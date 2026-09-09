use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("An escrow must trade two different mints")]
    SameMint,
    #[msg("Deadline must be in the future")]
    InvalidDeadline,
    #[msg("This escrow has expired")]
    Expired,
    #[msg("This escrow has not expired yet")]
    NotExpired,
    #[msg("The asking price changed since this transaction was built")]
    PriceChanged,
}
