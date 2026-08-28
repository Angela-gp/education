use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow amount must be greater than zero")]
    AmountMustBePositive,
    #[msg("Sender and receiver must be different")]
    SenderEqualsReceiver,
    #[msg("Escrow is not in the required status")]
    InvalidStatus,
    #[msg("Only the escrow sender may perform this action")]
    UnauthorizedSender,
    #[msg("The supplied receiver does not match the escrow")]
    ReceiverMismatch,
    #[msg("The supplied mint does not match the escrow")]
    MintMismatch,
    #[msg("Vault balance does not match the escrow amount")]
    UnexpectedVaultBalance,
    #[msg("Vault must be empty before it can be closed")]
    VaultNotEmpty,
}
