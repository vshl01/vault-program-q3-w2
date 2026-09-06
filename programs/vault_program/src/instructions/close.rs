use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{constants::*, state::Vault};

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority,
        close = authority,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        close = authority,
        associated_token::mint = vault_token_account.mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_close(_ctx: Context<Close>) -> Result<()> {
    Ok(())
}
