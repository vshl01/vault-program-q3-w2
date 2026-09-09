pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("GpvQhAe56FKFs9suG7PQ2bnxghikcwdWn7NbDVpN44Yt");

#[program]
pub mod nc_vault {
    use super::*;

    /// Opens a vault over one base asset and one underlying position.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handle_initialize(ctx)
    }

    /// Deposits base and mints shares pro-rata.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handle_deposit(ctx, amount)
    }

    /// Manager swaps liquid base into the position with a counterparty.
    pub fn deploy(ctx: Context<Deploy>, base_out: u64, position_in: u64) -> Result<()> {
        instructions::deploy::handle_deploy(ctx, base_out, position_in)
    }

    /// Exit at any time: burns shares for a pro-rata slice of base and position.
    pub fn redeem(ctx: Context<Redeem>, shares: u64) -> Result<()> {
        instructions::redeem::handle_redeem(ctx, shares)
    }
}
