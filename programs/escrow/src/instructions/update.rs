use anchor_lang::prelude::*;

use crate::{constants::*, error::EscrowError, state::Escrow};

#[derive(Accounts)]
pub struct Update<'info> {
    pub maker: Signer<'info>,

    #[account(
        mut,
        has_one = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), &escrow.seed.to_le_bytes()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
}

/// Repricing only — the deposited amount in the vault is untouched.
pub fn handle_update(ctx: Context<Update>, receive: u64) -> Result<()> {
    require!(receive > 0, EscrowError::InvalidAmount);
    require!(
        Clock::get()?.unix_timestamp <= ctx.accounts.escrow.deadline,
        EscrowError::Expired
    );

    ctx.accounts.escrow.receive = receive;
    Ok(())
}
