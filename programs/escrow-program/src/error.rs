use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("The escrow seed is invalid.")]
    InvalidSeed,
    #[msg("The expiration must be in the future.")]
    InvalidExpiration,
    #[msg("The deposit and requested amount must be greater than zero.")]
    InvalidAmount,
    #[msg("The maker and taker must be different accounts.")]
    MakerCannotBeTaker,
    #[msg("The escrow has expired.")]
    EscrowExpired,
    #[msg("The escrow has not expired yet.")]
    EscrowNotExpired,
    #[msg("The mint accounts must be different.")]
    SameMint,
}
