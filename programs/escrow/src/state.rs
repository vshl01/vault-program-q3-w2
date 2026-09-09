use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    /// Lets one maker run several escrows at once.
    pub seed: u64,
    pub maker: Pubkey,
    /// What the maker deposited.
    pub mint_a: Pubkey,
    /// What the maker wants back.
    pub mint_b: Pubkey,
    /// Amount of `mint_b` the taker must pay.
    pub receive: u64,
    pub bump: u8,
}
