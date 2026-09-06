use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{constants::*, state::Vault};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority,
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = vault_token_account.mint,
        associated_token::authority = authority,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let transfer_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.vault.to_account_info(),
    };

    let signer_seeds: &[&[u8]] = &[
        VAULT_SEED,
        ctx.accounts.authority.key.as_ref(),
        &[ctx.accounts.vault.bump],
    ];

    let signer_seeds_array = [signer_seeds];

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(), 
        transfer_accounts
    ).with_signer(&signer_seeds_array);

    token::transfer(cpi_ctx, amount)?;

    Ok(())
}
