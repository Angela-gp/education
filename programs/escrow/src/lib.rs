use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("4wBqpZM9xaSheZzJSMawUKKwhdpChKbZ5eu5ky4Vigw");

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, deal_id: u64, amount: u64) -> Result<()> {
        instructions::initialize::handler(ctx, deal_id, amount)
    }

    pub fn deposit(ctx: Context<Deposit>) -> Result<()> {
        instructions::deposit::handler(ctx)
    }

    pub fn release(ctx: Context<Release>) -> Result<()> {
        instructions::release::handler(ctx)
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        instructions::cancel::handler(ctx)
    }
}
