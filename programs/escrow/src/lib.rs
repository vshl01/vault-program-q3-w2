pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("BrZuxiD85Ur3A42qNiTajtKru55BNir4Qo1gaRSGxwov");

#[program]
pub mod escrow {
    use super::*;

    /// Maker locks `deposit` of mint_a and asks for `receive` of mint_b until `deadline`.
    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit: u64,
        receive: u64,
        deadline: i64,
    ) -> Result<()> {
        instructions::make::handle_make(ctx, seed, deposit, receive, deadline)
    }

    /// Taker pays `expected_receive` of mint_b and claims the vault. Before the deadline only.
    pub fn take(ctx: Context<Take>, expected_receive: u64) -> Result<()> {
        instructions::take::handle_take(ctx, expected_receive)
    }

    /// Maker takes the deposit back. After the deadline only.
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        instructions::refund::handle_refund(ctx)
    }

    /// Maker changes the asking price.
    pub fn update(ctx: Context<Update>, receive: u64) -> Result<()> {
        instructions::update::handle_update(ctx, receive)
    }
}
