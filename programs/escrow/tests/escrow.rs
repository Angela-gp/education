use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{Mint, TokenAccount},
};
use escrow::state::{EscrowState, EscrowStatus};
use litesvm::{types::FailedTransactionMetadata, LiteSVM};
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

const DECIMALS: u8 = 6;
const INITIAL_BALANCE: u64 = 1_000_000_000;
const DEAL_AMOUNT: u64 = 300_000_000;
const DEAL_ID: u64 = 42;

fn program_bytes(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy")
        .join(format!("{name}.so"));
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build programs with `anchor build --ignore-keys` before tests. Could not read {}: {error}",
            path.display()
        )
    })
}

fn instruction(
    accounts: impl ToAccountMetas,
    data: impl InstructionData,
    program_id: anchor_lang::prelude::Pubkey,
) -> Instruction {
    Instruction {
        program_id,
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
    Mint::try_deserialize_unchecked(&mut data).expect("mint must deserialize")
}

fn read_token_account(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> TokenAccount {
    let account = svm.get_account(address).expect("token account must exist");
    let mut data = account.data.as_slice();
    TokenAccount::try_deserialize_unchecked(&mut data).expect("token account must deserialize")
}

fn read_escrow(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> EscrowState {
    let account = svm.get_account(address).expect("escrow state must exist");
    let mut data = account.data.as_slice();
    EscrowState::try_deserialize(&mut data).expect("escrow state must deserialize")
}

struct TestContext {
    svm: LiteSVM,
    payer: Keypair,
    sender: Keypair,
    receiver: Keypair,
    other_receiver: Keypair,
    wrong_signer: Keypair,
    mint_authority: Keypair,
    mint: Keypair,
    other_mint: Keypair,
}

impl TestContext {
    fn new() -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program(escrow::ID, &program_bytes("escrow"))
            .expect("escrow program must load");
        svm.add_program(
            solana_level_1_token_starter::ID,
            &program_bytes("solana_level_1_token_starter"),
        )
        .expect("token starter program must load");

        let context = Self {
            svm,
            payer: Keypair::new(),
            sender: Keypair::new(),
            receiver: Keypair::new(),
            other_receiver: Keypair::new(),
            wrong_signer: Keypair::new(),
            mint_authority: Keypair::new(),
            mint: Keypair::new(),
            other_mint: Keypair::new(),
        };
        let mut context = context;
        for account in [
            &context.payer,
            &context.sender,
            &context.receiver,
            &context.other_receiver,
            &context.wrong_signer,
        ] {
            context
                .svm
                .airdrop(&account.pubkey(), 10_000_000_000)
                .expect("airdrop must succeed");
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

    fn escrow(&self, sender: &Keypair, deal_id: u64) -> anchor_lang::prelude::Pubkey {
        anchor_lang::prelude::Pubkey::find_program_address(
            &[b"escrow", sender.pubkey().as_ref(), &deal_id.to_le_bytes()],
            &escrow::ID,
        )
        .0
    }

    fn vault(&self, escrow_address: anchor_lang::prelude::Pubkey) -> anchor_lang::prelude::Pubkey {
        anchor_lang::prelude::Pubkey::find_program_address(
            &[b"vault", escrow_address.as_ref()],
            &escrow::ID,
        )
        .0
    }

    fn create_mint(&mut self, mint: &Keypair) {
        let ix = instruction(
            solana_level_1_token_starter::accounts::CreateToken {
                payer: self.payer.pubkey(),
                authority: self.mint_authority.pubkey(),
                mint: mint.pubkey(),
                token_program: self.token_program(),
                system_program: anchor_lang::system_program::ID,
            },
            solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS },
            solana_level_1_token_starter::ID,
        );
        send_success(
            &mut self.svm,
            &self.payer,
            ix,
            &[&self.payer, &self.mint_authority, mint],
            "create mint",
        );
    }

    fn create_ata(&mut self, owner: &Keypair, mint: &Keypair) {
        let ix = instruction(
            solana_level_1_token_starter::accounts::CreateTokenAccount {
                payer: self.payer.pubkey(),
                owner: owner.pubkey(),
                mint: mint.pubkey(),
                token_account: self.ata(owner, mint),
                token_program: self.token_program(),
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: anchor_lang::system_program::ID,
            },
            solana_level_1_token_starter::instruction::CreateTokenAccount {},
            solana_level_1_token_starter::ID,
        );
        send_success(&mut self.svm, &self.payer, ix, &[&self.payer], "create ATA");
    }

    fn mint_to(&mut self, mint: &Keypair, destination: anchor_lang::prelude::Pubkey, amount: u64) {
        let ix = instruction(
            solana_level_1_token_starter::accounts::MintTokens {
                authority: self.mint_authority.pubkey(),
                mint: mint.pubkey(),
                destination,
                token_program: self.token_program(),
            },
            solana_level_1_token_starter::instruction::MintTokens { amount },
            solana_level_1_token_starter::ID,
        );
        send_success(
            &mut self.svm,
            &self.payer,
            ix,
            &[&self.payer, &self.mint_authority],
            "mint tokens",
        );
    }

    fn create_primary_fixture(&mut self, balance: u64) {
        let mint = self.mint.insecure_clone();
        let sender = self.sender.insecure_clone();
        let receiver = self.receiver.insecure_clone();
        let other_receiver = self.other_receiver.insecure_clone();
        let wrong_signer = self.wrong_signer.insecure_clone();
        self.create_mint(&mint);
        self.create_ata(&sender, &mint);
        self.create_ata(&receiver, &mint);
        self.create_ata(&other_receiver, &mint);
        self.create_ata(&wrong_signer, &mint);
        let sender_ata = self.ata(&self.sender, &self.mint);
        self.mint_to(&mint, sender_ata, balance);
    }

    fn initialize_instruction(
        &self,
        sender: &Keypair,
        receiver: &Keypair,
        mint: &Keypair,
        deal_id: u64,
        amount: u64,
    ) -> Instruction {
        let escrow_address = self.escrow(sender, deal_id);
        instruction(
            escrow::accounts::Initialize {
                sender: sender.pubkey(),
                receiver: receiver.pubkey(),
                mint: mint.pubkey(),
                escrow: escrow_address,
                vault: self.vault(escrow_address),
                token_program: self.token_program(),
                system_program: anchor_lang::system_program::ID,
            },
            escrow::instruction::Initialize { deal_id, amount },
            escrow::ID,
        )
    }

    fn initialize(&mut self, deal_id: u64, amount: u64) {
        let ix =
            self.initialize_instruction(&self.sender, &self.receiver, &self.mint, deal_id, amount);
        send_success(
            &mut self.svm,
            &self.payer,
            ix,
            &[&self.payer, &self.sender],
            "initialize",
        );
    }

    fn deposit_instruction(
        &self,
        signer: &Keypair,
        mint: &Keypair,
        escrow_address: anchor_lang::prelude::Pubkey,
        source: anchor_lang::prelude::Pubkey,
    ) -> Instruction {
        instruction(
            escrow::accounts::Deposit {
                sender: signer.pubkey(),
                escrow: escrow_address,
                mint: mint.pubkey(),
                sender_token_account: source,
                vault: self.vault(escrow_address),
                token_program: self.token_program(),
            },
            escrow::instruction::Deposit {},
            escrow::ID,
        )
    }

    fn deposit(&mut self, deal_id: u64) {
        let escrow_address = self.escrow(&self.sender, deal_id);
        let ix = self.deposit_instruction(
            &self.sender,
            &self.mint,
            escrow_address,
            self.ata(&self.sender, &self.mint),
        );
        send_success(
            &mut self.svm,
            &self.payer,
            ix,
            &[&self.payer, &self.sender],
            "deposit",
        );
    }

    fn release_instruction(
        &self,
        signer: &Keypair,
        receiver: &Keypair,
        mint: &Keypair,
        escrow_address: anchor_lang::prelude::Pubkey,
    ) -> Instruction {
        instruction(
            escrow::accounts::Release {
                sender: signer.pubkey(),
                receiver: receiver.pubkey(),
                escrow: escrow_address,
                mint: mint.pubkey(),
                vault: self.vault(escrow_address),
                receiver_token_account: self.ata(receiver, mint),
                token_program: self.token_program(),
                associated_token_program: anchor_spl::associated_token::ID,
            },
            escrow::instruction::Release {},
            escrow::ID,
        )
    }

    fn cancel_instruction(
        &self,
        signer: &Keypair,
        mint: &Keypair,
        escrow_address: anchor_lang::prelude::Pubkey,
    ) -> Instruction {
        instruction(
            escrow::accounts::Cancel {
                sender: signer.pubkey(),
                escrow: escrow_address,
                mint: mint.pubkey(),
                vault: self.vault(escrow_address),
                sender_token_account: self.ata(signer, mint),
                token_program: self.token_program(),
                associated_token_program: anchor_spl::associated_token::ID,
            },
            escrow::instruction::Cancel {},
            escrow::ID,
        )
    }
}

#[test]
fn releases_funded_escrow_end_to_end_and_rejects_repeat() {
    let mut context = TestContext::new();
    context.create_primary_fixture(INITIAL_BALANCE);
    context.initialize(DEAL_ID, DEAL_AMOUNT);
    let escrow_address = context.escrow(&context.sender, DEAL_ID);
    let vault = context.vault(escrow_address);
    let sender_ata = context.ata(&context.sender, &context.mint);
    let receiver_ata = context.ata(&context.receiver, &context.mint);
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;

    let created = read_escrow(&context.svm, &escrow_address);
    assert_eq!(created.sender, context.sender.pubkey());
    assert_eq!(created.receiver, context.receiver.pubkey());
    assert_eq!(created.mint, context.mint.pubkey());
    assert_eq!(created.amount, DEAL_AMOUNT);
    assert_eq!(created.deal_id, DEAL_ID);
    assert_eq!(created.status, EscrowStatus::Created);

    context.deposit(DEAL_ID);
    assert_eq!(
        read_escrow(&context.svm, &escrow_address).status,
        EscrowStatus::Funded
    );
    assert_eq!(read_token_account(&context.svm, &vault).amount, DEAL_AMOUNT);
    assert_eq!(
        read_token_account(&context.svm, &sender_ata).amount,
        INITIAL_BALANCE - DEAL_AMOUNT
    );

    let release = context.release_instruction(
        &context.sender,
        &context.receiver,
        &context.mint,
        escrow_address,
    );
    send_success(
        &mut context.svm,
        &context.payer,
        release.clone(),
        &[&context.payer, &context.sender],
        "release",
    );

    assert_eq!(
        read_token_account(&context.svm, &receiver_ata).amount,
        DEAL_AMOUNT
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
    assert!(context.svm.get_account(&escrow_address).is_none());
    assert!(context.svm.get_account(&vault).is_none());

    let receiver_before = read_token_account(&context.svm, &receiver_ata).amount;
    send_failure(
        &mut context.svm,
        &context.payer,
        release,
        &[&context.payer, &context.sender],
        "repeated release",
    );
    assert_eq!(
        read_token_account(&context.svm, &receiver_ata).amount,
        receiver_before
    );
}

#[test]
fn cancels_funded_escrow_end_to_end_and_rejects_repeat() {
    let mut context = TestContext::new();
    context.create_primary_fixture(INITIAL_BALANCE);
    context.initialize(DEAL_ID, DEAL_AMOUNT);
    context.deposit(DEAL_ID);
    let escrow_address = context.escrow(&context.sender, DEAL_ID);
    let vault = context.vault(escrow_address);
    let sender_ata = context.ata(&context.sender, &context.mint);
    let supply_before = read_mint(&context.svm, &context.mint.pubkey()).supply;
    let cancel = context.cancel_instruction(&context.sender, &context.mint, escrow_address);

    send_success(
        &mut context.svm,
        &context.payer,
        cancel.clone(),
        &[&context.payer, &context.sender],
        "cancel",
    );
    assert_eq!(
        read_token_account(&context.svm, &sender_ata).amount,
        INITIAL_BALANCE
    );
    assert_eq!(
        read_mint(&context.svm, &context.mint.pubkey()).supply,
        supply_before
    );
    assert!(context.svm.get_account(&escrow_address).is_none());
    assert!(context.svm.get_account(&vault).is_none());

    send_failure(
        &mut context.svm,
        &context.payer,
        cancel,
        &[&context.payer, &context.sender],
        "repeated cancel",
    );
    assert_eq!(
        read_token_account(&context.svm, &sender_ata).amount,
        INITIAL_BALANCE
    );
}

#[test]
fn rejects_invalid_initialize_without_creating_accounts() {
    let mut context = TestContext::new();
    context.create_primary_fixture(INITIAL_BALANCE);

    let zero_id = DEAL_ID + 1;
    let zero_escrow = context.escrow(&context.sender, zero_id);
    let zero = context.initialize_instruction(
        &context.sender,
        &context.receiver,
        &context.mint,
        zero_id,
        0,
    );
    let failure = send_failure(
        &mut context.svm,
        &context.payer,
        zero,
        &[&context.payer, &context.sender],
        "zero amount initialize",
    );
    assert!(failure
        .meta
        .logs
        .iter()
        .any(|log| log.contains("Escrow amount must be greater than zero")));
    assert!(context.svm.get_account(&zero_escrow).is_none());
    assert!(context
        .svm
        .get_account(&context.vault(zero_escrow))
        .is_none());

    let same_id = DEAL_ID + 2;
    let same_escrow = context.escrow(&context.sender, same_id);
    let same_party = context.initialize_instruction(
        &context.sender,
        &context.sender,
        &context.mint,
        same_id,
        DEAL_AMOUNT,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        same_party,
        &[&context.payer, &context.sender],
        "same sender and receiver",
    );
    assert!(context.svm.get_account(&same_escrow).is_none());
}

#[test]
fn rejects_duplicate_deal_id_without_changing_existing_state() {
    let mut context = TestContext::new();
    context.create_primary_fixture(INITIAL_BALANCE);
    context.initialize(DEAL_ID, DEAL_AMOUNT);
    let escrow_address = context.escrow(&context.sender, DEAL_ID);
    let vault = context.vault(escrow_address);
    let state_before = read_escrow(&context.svm, &escrow_address);
    let vault_before = read_token_account(&context.svm, &vault).amount;
    let duplicate = context.initialize_instruction(
        &context.sender,
        &context.receiver,
        &context.mint,
        DEAL_ID,
        DEAL_AMOUNT,
    );

    send_failure(
        &mut context.svm,
        &context.payer,
        duplicate,
        &[&context.payer, &context.sender],
        "duplicate deal id",
    );
    assert_eq!(read_escrow(&context.svm, &escrow_address), state_before);
    assert_eq!(
        read_token_account(&context.svm, &vault).amount,
        vault_before
    );
}

#[test]
fn rejects_wrong_signer_and_insufficient_deposit_without_state_change() {
    let mut context = TestContext::new();
    context.create_primary_fixture(DEAL_AMOUNT - 1);
    context.initialize(DEAL_ID, DEAL_AMOUNT);
    let escrow_address = context.escrow(&context.sender, DEAL_ID);
    let vault = context.vault(escrow_address);
    let sender_ata = context.ata(&context.sender, &context.mint);
    let state_before = read_escrow(&context.svm, &escrow_address);
    let source_before = read_token_account(&context.svm, &sender_ata).amount;

    let wrong = context.deposit_instruction(
        &context.wrong_signer,
        &context.mint,
        escrow_address,
        sender_ata,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        wrong,
        &[&context.payer, &context.wrong_signer],
        "deposit by wrong signer",
    );
    let insufficient =
        context.deposit_instruction(&context.sender, &context.mint, escrow_address, sender_ata);
    send_failure(
        &mut context.svm,
        &context.payer,
        insufficient,
        &[&context.payer, &context.sender],
        "deposit with insufficient balance",
    );

    assert_eq!(read_escrow(&context.svm, &escrow_address), state_before);
    assert_eq!(read_token_account(&context.svm, &vault).amount, 0);
    assert_eq!(
        read_token_account(&context.svm, &sender_ata).amount,
        source_before
    );
}

#[test]
fn rejects_mint_and_receiver_substitution_without_state_change() {
    let mut context = TestContext::new();
    context.create_primary_fixture(INITIAL_BALANCE);
    let other_mint = context.other_mint.insecure_clone();
    let sender = context.sender.insecure_clone();
    let receiver = context.receiver.insecure_clone();
    context.create_mint(&other_mint);
    context.create_ata(&sender, &other_mint);
    context.create_ata(&receiver, &other_mint);
    context.initialize(DEAL_ID, DEAL_AMOUNT);
    let escrow_address = context.escrow(&context.sender, DEAL_ID);
    let vault = context.vault(escrow_address);

    let wrong_mint_deposit = context.deposit_instruction(
        &context.sender,
        &context.other_mint,
        escrow_address,
        context.ata(&context.sender, &context.other_mint),
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        wrong_mint_deposit,
        &[&context.payer, &context.sender],
        "deposit with substituted mint",
    );
    assert_eq!(
        read_escrow(&context.svm, &escrow_address).status,
        EscrowStatus::Created
    );
    assert_eq!(read_token_account(&context.svm, &vault).amount, 0);

    context.deposit(DEAL_ID);
    let funded_before = read_escrow(&context.svm, &escrow_address);
    let vault_before = read_token_account(&context.svm, &vault).amount;
    let wrong_receiver = context.release_instruction(
        &context.sender,
        &context.other_receiver,
        &context.mint,
        escrow_address,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        wrong_receiver,
        &[&context.payer, &context.sender],
        "release with substituted receiver",
    );
    let wrong_mint_release = context.release_instruction(
        &context.sender,
        &context.receiver,
        &context.other_mint,
        escrow_address,
    );
    send_failure(
        &mut context.svm,
        &context.payer,
        wrong_mint_release,
        &[&context.payer, &context.sender],
        "release with substituted mint",
    );
    assert_eq!(read_escrow(&context.svm, &escrow_address), funded_before);
    assert_eq!(
        read_token_account(&context.svm, &vault).amount,
        vault_before
    );
}

#[test]
fn rejects_duplicate_deposit_without_changing_funded_state() {
    let mut context = TestContext::new();
    context.create_primary_fixture(INITIAL_BALANCE);
    context.initialize(DEAL_ID, DEAL_AMOUNT);
    context.deposit(DEAL_ID);
    let escrow_address = context.escrow(&context.sender, DEAL_ID);
    let vault = context.vault(escrow_address);
    let sender_ata = context.ata(&context.sender, &context.mint);
    let state_before = read_escrow(&context.svm, &escrow_address);
    let vault_before = read_token_account(&context.svm, &vault).amount;
    let sender_before = read_token_account(&context.svm, &sender_ata).amount;
    let duplicate =
        context.deposit_instruction(&context.sender, &context.mint, escrow_address, sender_ata);

    send_failure(
        &mut context.svm,
        &context.payer,
        duplicate,
        &[&context.payer, &context.sender],
        "duplicate deposit",
    );
    assert_eq!(read_escrow(&context.svm, &escrow_address), state_before);
    assert_eq!(
        read_token_account(&context.svm, &vault).amount,
        vault_before
    );
    assert_eq!(
        read_token_account(&context.svm, &sender_ata).amount,
        sender_before
    );
}

#[test]
fn rejects_unauthorized_cancel_without_changing_funded_state() {
    let mut context = TestContext::new();
    context.create_primary_fixture(INITIAL_BALANCE);
    context.initialize(DEAL_ID, DEAL_AMOUNT);
    context.deposit(DEAL_ID);
    let escrow_address = context.escrow(&context.sender, DEAL_ID);
    let vault = context.vault(escrow_address);
    let state_before = read_escrow(&context.svm, &escrow_address);
    let vault_before = read_token_account(&context.svm, &vault).amount;
    let wrong_cancel =
        context.cancel_instruction(&context.wrong_signer, &context.mint, escrow_address);

    send_failure(
        &mut context.svm,
        &context.payer,
        wrong_cancel,
        &[&context.payer, &context.wrong_signer],
        "cancel by wrong signer",
    );
    assert_eq!(read_escrow(&context.svm, &escrow_address), state_before);
    assert_eq!(
        read_token_account(&context.svm, &vault).amount,
        vault_before
    );
}
