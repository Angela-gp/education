use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{
    error::EscrowError,
    state::{EscrowState, EscrowStatus},
};

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        mut,
        close = sender,
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
        seeds = [b"vault", escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = sender,
        associated_token::token_program = token_program,
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn handler(ctx: Context<Cancel>) -> Result<()> {
    require!(
        matches!(
            ctx.accounts.escrow.status,
            EscrowStatus::Created | EscrowStatus::Funded
        ),
        EscrowError::InvalidStatus
    );

    let sender = ctx.accounts.escrow.sender;
    let deal_id = ctx.accounts.escrow.deal_id.to_le_bytes();
    let bump = [ctx.accounts.escrow.bump];
    let signer_seeds: &[&[u8]] = &[b"escrow", sender.as_ref(), &deal_id, &bump];
    let signer = &[signer_seeds];
    let amount = ctx.accounts.vault.amount;

    if amount > 0 {
        let transfer_accounts = TransferChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.sender_token_account.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        };
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                transfer_accounts,
                signer,
            ),
            amount,
            ctx.accounts.mint.decimals,
        )?;
        ctx.accounts.vault.reload()?;
    }

    require_eq!(ctx.accounts.vault.amount, 0, EscrowError::VaultNotEmpty);
    ctx.accounts.escrow.status = EscrowStatus::Cancelled;

    let close_accounts = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.sender.to_account_info(),
        authority: ctx.accounts.escrow.to_account_info(),
    };
    token_interface::close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        close_accounts,
        signer,
    ))
}
