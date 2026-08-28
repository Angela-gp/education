use anchor_lang::{
    solana_program::program_option::COption, AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{Mint, TokenAccount},
};
use litesvm::{types::FailedTransactionMetadata, LiteSVM};
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

const DECIMALS: u8 = 6;
const INITIAL_MINT_AMOUNT: u64 = 1_000_000_000;
const TRANSFER_AMOUNT: u64 = 125_000_000;
const BURN_AMOUNT: u64 = 200_000_000;

fn program_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/solana_level_1_token_starter.so");
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build the program with `anchor build --ignore-keys` before running tests. Could not read {}: {error}",
            path.display()
        )
    })
}

fn instruction(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: data.data(),
    }
}

fn transaction(
    svm: &LiteSVM,
    payer: &Keypair,
    instruction: Instruction,
    signers: &[&dyn Signer],
) -> Transaction {
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    Transaction::new(signers, message, svm.latest_blockhash())
}

fn send_success(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instruction: Instruction,
    signers: &[&dyn Signer],
    operation: &str,
) {
    let transaction = transaction(svm, payer, instruction, signers);
    svm.send_transaction(transaction)
        .unwrap_or_else(|error| panic!("{operation} must succeed: {error:?}"));
}

fn send_failure(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instruction: Instruction,
    signers: &[&dyn Signer],
    operation: &str,
) -> FailedTransactionMetadata {
    let transaction = transaction(svm, payer, instruction, signers);
    svm.send_transaction(transaction)
        .expect_err(&format!("{operation} must fail"))
}

fn read_mint(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> Mint {
    let account = svm.get_account(address).expect("mint must exist");
    let mut data = account.data.as_slice();
    Mint::try_deserialize_unchecked(&mut data).expect("mint data must deserialize")
}

fn read_token_account(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> TokenAccount {
    let account = svm.get_account(address).expect("token account must exist");
    let mut data = account.data.as_slice();
    TokenAccount::try_deserialize_unchecked(&mut data).expect("token account data must deserialize")
}

struct TestContext {
    svm: LiteSVM,
    payer: Keypair,
    authority: Keypair,
    wrong_authority: Keypair,
    alice: Keypair,
    bob: Keypair,
    mint: Keypair,
    other_mint: Keypair,
}

impl TestContext {
    fn new() -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program(solana_level_1_token_starter::ID, &program_bytes())
            .expect("program must load");

        let mut context = Self {
            svm,
            payer: Keypair::new(),
            authority: Keypair::new(),
            wrong_authority: Keypair::new(),
            alice: Keypair::new(),
            bob: Keypair::new(),
            mint: Keypair::new(),
            other_mint: Keypair::new(),
        };

        context
            .svm
            .airdrop(&context.payer.pubkey(), 10_000_000_000)
            .expect("payer airdrop must succeed");
        for owner in [&context.alice, &context.bob] {
            context
                .svm
                .airdrop(&owner.pubkey(), 1_000_000)
                .expect("owner airdrop must succeed");
        }

        context
    }

    fn token_program(&self) -> anchor_lang::prelude::Pubkey {
        anchor_spl::token_2022::ID
    }

    fn ata(&self, owner: &Keypair, mint: &Keypair) -> anchor_lang::prelude::Pubkey {
        get_associated_token_address_with_program_id(
            &owner.pubkey(),
            &mint.pubkey(),
            &self.token_program(),
        )
    }

    fn create_mint(&mut self, mint: &Keypair) {
        let accounts = solana_level_1_token_starter::accounts::CreateToken {
            payer: self.payer.pubkey(),
            authority: self.authority.pubkey(),
            mint: mint.pubkey(),
            token_program: self.token_program(),
            system_program: anchor_lang::system_program::ID,
        };
        let instruction = instruction(
            accounts,
            solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS },
        );
        send_success(
            &mut self.svm,
            &self.payer,
            instruction,
            &[&self.payer, &self.authority, mint],
            "create_token",
        );
    }

    fn create_ata(&mut self, owner: &Keypair, mint: &Keypair) {
        let accounts = solana_level_1_token_starter::accounts::CreateTokenAccount {
            payer: self.payer.pubkey(),
            owner: owner.pubkey(),
            mint: mint.pubkey(),
            token_account: self.ata(owner, mint),
            token_program: self.token_program(),
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        };
        let instruction = instruction(
            accounts,
            solana_level_1_token_starter::instruction::CreateTokenAccount {},
        );
        send_success(
            &mut self.svm,
            &self.payer,
            instruction,
            &[&self.payer],
            "create_token_account",
        );
    }

    fn mint_instruction(
        &self,
        authority: &Keypair,
        mint: &Keypair,
        destination: anchor_lang::prelude::Pubkey,
        amount: u64,
    ) -> Instruction {
        instruction(
            solana_level_1_token_starter::accounts::MintTokens {
                authority: authority.pubkey(),
                mint: mint.pubkey(),
                destination,
                token_program: self.token_program(),
            },
            solana_level_1_token_starter::instruction::MintTokens { amount },
        )
    }

    fn mint_tokens(&mut self, destination: anchor_lang::prelude::Pubkey, amount: u64) {
        let instruction = self.mint_instruction(&self.authority, &self.mint, destination, amount);
        send_success(
            &mut self.svm,
            &self.payer,
            instruction,
            &[&self.payer, &self.authority],
            "mint_tokens",
        );
    }

    fn transfer_instruction(
        &self,
        authority: &Keypair,
        mint: &Keypair,
        source: anchor_lang::prelude::Pubkey,
        destination: anchor_lang::prelude::Pubkey,
        amount: u64,
    ) -> Instruction {
        instruction(
            solana_level_1_token_starter::accounts::TransferTokens {
                authority: authority.pubkey(),
                mint: mint.pubkey(),
                source,
                destination,
                token_program: self.token_program(),
            },
            solana_level_1_token_starter::instruction::TransferTokens { amount },
        )
    }

    fn burn_instruction(
        &self,
        authority: &Keypair,
        mint: &Keypair,
        token_account: anchor_lang::prelude::Pubkey,
        amount: u64,
    ) -> Instruction {
        instruction(
            solana_level_1_token_starter::accounts::BurnTokens {
                authority: authority.pubkey(),
                mint: mint.pubkey(),
                token_account,
                token_program: self.token_program(),
            },
            solana_level_1_token_starter::instruction::BurnTokens { amount },
        )
    }

    fn create_primary_fixture(&mut self) {
        let mint = self.mint.insecure_clone();
        let alice = self.alice.insecure_clone();
        let bob = self.bob.insecure_clone();
        self.create_mint(&mint);
        self.create_ata(&alice, &mint);
        self.create_ata(&bob, &mint);
    }
}

#[test]
fn creates_token_2022_mint_with_expected_state() {
    let mut context = TestContext::new();
    let mint = context.mint.insecure_clone();
    context.create_mint(&mint);

    let raw_mint = context
        .svm
        .get_account(&context.mint.pubkey())
        .expect("mint must exist");
    let mint = read_mint(&context.svm, &context.mint.pubkey());

    assert_eq!(raw_mint.owner, context.token_program());
    assert_eq!(mint.decimals, DECIMALS);
    assert_eq!(
        mint.mint_authority,
        COption::Some(context.authority.pubkey())
    );
    assert_eq!(mint.supply, 0);
}

#[test]
fn creates_token_2022_account_with_expected_state() {
    let mut context = TestContext::new();
    let mint = context.mint.insecure_clone();
    let alice = context.alice.insecure_clone();
    context.create_mint(&mint);
    context.create_ata(&alice, &mint);

    let address = context.ata(&context.alice, &context.mint);
    let raw_account = context
        .svm
        .get_account(&address)
        .expect("token account must exist");
    let token_account = read_token_account(&context.svm, &address);

    assert_eq!(raw_account.owner, context.token_program());
    assert_eq!(token_account.owner, context.alice.pubkey());
    assert_eq!(token_account.mint, context.mint.pubkey());
    assert_eq!(token_account.amount, 0);
}

#[test]
fn mints_tokens_and_increases_destination_and_supply() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);

    let balance_before = read_token_account(&context.svm, &alice_ata).amount;
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        balance_before + INITIAL_MINT_AMOUNT
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before + INITIAL_MINT_AMOUNT
    );
}

#[test]
fn transfers_tokens_between_accounts_without_changing_supply() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    let bob_ata = context.ata(&context.bob, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let source_before = read_token_account(&context.svm, &alice_ata).amount;
    let destination_before = read_token_account(&context.svm, &bob_ata).amount;
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let instruction = context.transfer_instruction(
        &context.alice,
        &context.mint,
        alice_ata,
        bob_ata,
        TRANSFER_AMOUNT,
    );
    send_success(
        &mut context.svm,
        &context.payer,
        instruction,
        &[&context.payer, &context.alice],
        "transfer_tokens",
    );

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        source_before - TRANSFER_AMOUNT
    );
    assert_eq!(
        read_token_account(&context.svm, &bob_ata).amount,
        destination_before + TRANSFER_AMOUNT
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
}

#[test]
fn rejects_zero_mint_and_transfer_amounts() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    let bob_ata = context.ata(&context.bob, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let alice_before = read_token_account(&context.svm, &alice_ata).amount;
    let bob_before = read_token_account(&context.svm, &bob_ata).amount;

    let mint_instruction =
        context.mint_instruction(&context.authority, &context.mint, alice_ata, 0);
    send_failure(
        &mut context.svm,
        &context.payer,
        mint_instruction,
        &[&context.payer, &context.authority],
        "mint_tokens with zero amount",
    );
    let transfer_instruction =
        context.transfer_instruction(&context.alice, &context.mint, alice_ata, bob_ata, 0);
    send_failure(
        &mut context.svm,
        &context.payer,
        transfer_instruction,
        &[&context.payer, &context.alice],
        "transfer_tokens with zero amount",
    );

    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        alice_before
    );
    assert_eq!(
        read_token_account(&context.svm, &bob_ata).amount,
        bob_before
    );
}

#[test]
fn rejects_wrong_mint_and_transfer_authorities() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    let bob_ata = context.ata(&context.bob, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let alice_before = read_token_account(&context.svm, &alice_ata).amount;
    let bob_before = read_token_account(&context.svm, &bob_ata).amount;

    let mint_instruction = context.mint_instruction(
        &context.wrong_authority,
        &context.mint,
        alice_ata,
        TRANSFER_AMOUNT,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        mint_instruction,
        &[&context.payer, &context.wrong_authority],
        "mint_tokens with wrong authority",
    );
    let transfer_instruction = context.transfer_instruction(
        &context.wrong_authority,
        &context.mint,
        alice_ata,
        bob_ata,
        TRANSFER_AMOUNT,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        transfer_instruction,
        &[&context.payer, &context.wrong_authority],
        "transfer_tokens with wrong authority",
    );

    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        alice_before
    );
    assert_eq!(
        read_token_account(&context.svm, &bob_ata).amount,
        bob_before
    );
}

#[test]
fn rejects_token_accounts_from_another_mint() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let other_mint = context.other_mint.insecure_clone();
    let bob = context.bob.insecure_clone();
    context.create_mint(&other_mint);
    context.create_ata(&bob, &other_mint);

    let alice_ata = context.ata(&context.alice, &context.mint);
    let other_bob_ata = context.ata(&context.bob, &context.other_mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);
    let primary_supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let other_supply_before = read_mint(&context.svm, &context.other_mint.pubkey()).supply;
    let alice_before = read_token_account(&context.svm, &alice_ata).amount;
    let other_bob_before = read_token_account(&context.svm, &other_bob_ata).amount;

    let mint_instruction = context.mint_instruction(
        &context.authority,
        &context.mint,
        other_bob_ata,
        TRANSFER_AMOUNT,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        mint_instruction,
        &[&context.payer, &context.authority],
        "mint_tokens to an account for another mint",
    );
    let transfer_instruction = context.transfer_instruction(
        &context.alice,
        &context.mint,
        alice_ata,
        other_bob_ata,
        TRANSFER_AMOUNT,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        transfer_instruction,
        &[&context.payer, &context.alice],
        "transfer_tokens to an account for another mint",
    );

    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        primary_supply_before
    );
    assert_eq!(
        read_mint(&context.svm, &context.other_mint.pubkey()).supply,
        other_supply_before
    );
    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        alice_before
    );
    assert_eq!(
        read_token_account(&context.svm, &other_bob_ata).amount,
        other_bob_before
    );
}

#[test]
fn rejects_transfer_to_the_source_account() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let balance_before = read_token_account(&context.svm, &alice_ata).amount;
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let instruction = context.transfer_instruction(
        &context.alice,
        &context.mint,
        alice_ata,
        alice_ata,
        TRANSFER_AMOUNT,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        instruction,
        &[&context.payer, &context.alice],
        "transfer_tokens with identical source and destination",
    );

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        balance_before
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
}

#[test]
fn burns_tokens_and_decreases_balance_and_supply_equally() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let balance_before = read_token_account(&context.svm, &alice_ata).amount;
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let instruction =
        context.burn_instruction(&context.alice, &context.mint, alice_ata, BURN_AMOUNT);
    send_success(
        &mut context.svm,
        &context.payer,
        instruction,
        &[&context.payer, &context.alice],
        "burn_tokens",
    );

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        balance_before - BURN_AMOUNT
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before - BURN_AMOUNT
    );
}

#[test]
fn rejects_zero_burn_without_changing_state() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let balance_before = read_token_account(&context.svm, &alice_ata).amount;
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let instruction = context.burn_instruction(&context.alice, &context.mint, alice_ata, 0);
    let failure = send_failure(
        &mut context.svm,
        &context.payer,
        instruction,
        &[&context.payer, &context.alice],
        "burn_tokens with zero amount",
    );
    assert!(
        failure
            .meta
            .logs
            .iter()
            .any(|log| log.contains("Amount must be greater than zero")),
        "zero burn must return AmountMustBePositive: {:?}",
        failure.meta.logs
    );

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        balance_before
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
}

#[test]
fn rejects_burn_by_wrong_authority_without_changing_state() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let balance_before = read_token_account(&context.svm, &alice_ata).amount;
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let instruction = context.burn_instruction(
        &context.wrong_authority,
        &context.mint,
        alice_ata,
        BURN_AMOUNT,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        instruction,
        &[&context.payer, &context.wrong_authority],
        "burn_tokens with wrong authority",
    );

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        balance_before
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
}

#[test]
fn rejects_burn_with_another_mint_without_changing_state() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let other_mint = context.other_mint.insecure_clone();
    context.create_mint(&other_mint);
    let alice_ata = context.ata(&context.alice, &context.mint);
    context.mint_tokens(alice_ata, INITIAL_MINT_AMOUNT);

    let balance_before = read_token_account(&context.svm, &alice_ata).amount;
    let primary_supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let other_supply_before = read_mint(&context.svm, &context.other_mint.pubkey()).supply;
    let instruction =
        context.burn_instruction(&context.alice, &context.other_mint, alice_ata, BURN_AMOUNT);
    send_failure(
        &mut context.svm,
        &context.payer,
        instruction,
        &[&context.payer, &context.alice],
        "burn_tokens with another mint",
    );

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        balance_before
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        primary_supply_before
    );
    assert_eq!(
        read_mint(&context.svm, &context.other_mint.pubkey()).supply,
        other_supply_before
    );
}

#[test]
fn rejects_burn_above_balance_without_changing_state() {
    let mut context = TestContext::new();
    context.create_primary_fixture();
    let alice_ata = context.ata(&context.alice, &context.mint);
    context.mint_tokens(alice_ata, BURN_AMOUNT);

    let balance_before = read_token_account(&context.svm, &alice_ata).amount;
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let instruction =
        context.burn_instruction(&context.alice, &context.mint, alice_ata, balance_before + 1);
    send_failure(
        &mut context.svm,
        &context.payer,
        instruction,
        &[&context.payer, &context.alice],
        "burn_tokens above available balance",
    );

    assert_eq!(
        read_token_account(&context.svm, &alice_ata).amount,
        balance_before
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
}
