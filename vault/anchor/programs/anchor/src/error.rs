use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Vault already exists")]
    VaultAlreadyExists,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Non-zero balance")]
    NonZeroBalance,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid argument")]
    InvalidArgument,
    #[msg("Custom error message")]
    CustomError,
}
