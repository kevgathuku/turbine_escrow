use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Deposit amount must be greater than zero")]
    InvalidDeposit,
    #[msg("Receive amount must be greater than zero")]
    InvalidReceive,
    #[msg("Invalid maker")]
    InvalidMaker,
    #[msg("Invalid mint A")]
    InvalidMintA,
    #[msg("Invalid mint B")]
    InvalidMintB,
    #[msg("Insufficient token balance to complete the swap")]
    InsufficientBalance,
    // #[msg("The escrow has expired")]
    // EscrowExpired,
}
