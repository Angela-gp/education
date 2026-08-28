use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, PartialEq, Eq)]
pub enum EscrowStatus {
    Created,
    Funded,
    Released,
    Cancelled,
}

#[account]
#[derive(Debug, InitSpace, PartialEq, Eq)]
pub struct EscrowState {
    pub sender: Pubkey,
    pub receiver: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub deal_id: u64,
    pub bump: u8,
    pub status: EscrowStatus,
}
