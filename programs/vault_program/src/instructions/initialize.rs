use anchor_lang::prelude::*;

use crate::{constants::*, state::Vault};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init, 
        payer = authority, 
        space = 8 + Vault::INIT_SPACE, 
        seeds = [VAULT_SEED, authority.key().as_ref()], 
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;

    vault.authority = ctx.accounts.authority.key();
    vault.bump = ctx.bumps.vault;

    Ok(())
}
