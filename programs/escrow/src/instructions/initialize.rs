use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    error::EscrowError,
    state::{EscrowState, EscrowStatus},
};

#[derive(Accounts)]
#[instruction(deal_id: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        constraint = sender.key() != receiver.key() @ EscrowError::SenderEqualsReceiver,
    )]
    pub receiver: SystemAccount<'info>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = sender,
        space = 8 + EscrowState::INIT_SPACE,
        seeds = [b"escrow", sender.key().as_ref(), &deal_id.to_le_bytes()],
        bump,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account(
        init,
        payer = sender,
        seeds = [b"vault", escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, deal_id: u64, amount: u64) -> Result<()> {
    require!(amount > 0, EscrowError::AmountMustBePositive);

    ctx.accounts.escrow.set_inner(EscrowState {
        sender: ctx.accounts.sender.key(),
        receiver: ctx.accounts.receiver.key(),
        mint: ctx.accounts.mint.key(),
        amount,
        deal_id,
        bump: ctx.bumps.escrow,
        status: EscrowStatus::Created,
    });
    Ok(())
}
