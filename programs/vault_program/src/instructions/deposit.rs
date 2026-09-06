use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{state::Vault, VAULT_SEED};
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = vault_token_account.mint,
        associated_token::authority = authority,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_SEED, authority.key().as_ref()],
        bump = vault.bump,
        has_one = authority,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        associated_token::mint = user_token_account.mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    // here `Transfer` is the instruction from spl token program we are just defining context for cpi
    // `Transfer` defines the accounts required by the SPL Token transfer instruction.
    // For this SPL Token transfer instruction, here are the accounts it needs."
    /*
        Transfer {
            from      → where tokens come from
            to        → where tokens go
            authority → who is allowed to move them
        }
    */

    let transfer_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    // Creates the CPI context: which program to call + which accounts to pass.
    // This does NOT execute the CPI.
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), transfer_accounts);

    // Executes the CPI to the SPL Token Program.
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}
