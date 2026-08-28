use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{
    error::EscrowError,
    state::{EscrowState, EscrowStatus},
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub sender: Signer<'info>,
    #[account(
        mut,
        seeds = [b"escrow", sender.key().as_ref(), &escrow.deal_id.to_le_bytes()],
        bump = escrow.bump,
        has_one = sender @ EscrowError::UnauthorizedSender,
        has_one = mint @ EscrowError::MintMismatch,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = sender,
        token::token_program = token_program,
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"vault", escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Deposit>) -> Result<()> {
    require!(
        ctx.accounts.escrow.status == EscrowStatus::Created,
        EscrowError::InvalidStatus
    );
    require_eq!(
        ctx.accounts.vault.amount,
        0,
        EscrowError::UnexpectedVaultBalance
    );

    let amount = ctx.accounts.escrow.amount;
    let decimals = ctx.accounts.mint.decimals;
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.sender_token_account.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.sender.to_account_info(),
    };
    token_interface::transfer_checked(
        CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts),
        amount,
        decimals,
    )?;

    ctx.accounts.vault.reload()?;
    require_eq!(
        ctx.accounts.vault.amount,
        amount,
        EscrowError::UnexpectedVaultBalance
    );
    ctx.accounts.escrow.status = EscrowStatus::Funded;
    Ok(())
}
