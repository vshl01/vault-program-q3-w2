use anchor_lang::prelude::*;

/// A vault over one base asset and one underlying market position.
///
/// Share supply lives on `share_mint`, so it can never disagree with what
/// holders actually own.
#[account]
#[derive(InitSpace)]
pub struct Vault {
    /// May move liquidity into the position. Can never move assets out to itself.
    pub manager: Pubkey,
    /// The liquid asset users deposit.
    pub base_mint: Pubkey,
    /// The underlying market position, handed out in kind on redemption.
    pub position_mint: Pubkey,
    pub share_mint: Pubkey,
    pub bump: u8,
}
