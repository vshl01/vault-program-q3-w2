use anchor_lang::prelude::*;

use crate::{constants::*, state::Vault};

use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;

#[derive(Accounts)]
pub struct Initialize<'info> {
    // vault pda handling
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

    // vault token account handling
    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn handle_initialize(ctx: Context<Initialize>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;

    vault.authority = ctx.accounts.authority.key();
    vault.bump = ctx.bumps.vault;

    Ok(())
}
