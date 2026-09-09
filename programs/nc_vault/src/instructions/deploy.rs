use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{constants::*, error::VaultError, state::Vault};

/// Manager moves liquid base into the underlying position by swapping with a
/// counterparty. Both legs settle here, and neither can pay the manager.
#[derive(Accounts)]
pub struct Deploy<'info> {
    pub manager: Signer<'info>,
    pub counterparty: Signer<'info>,

    #[account(
        has_one = manager,
        has_one = base_mint,
        has_one = position_mint,
        seeds = [VAULT_SEED, base_mint.key().as_ref(), position_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub base_mint: Box<Account<'info, Mint>>,
    pub position_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = vault,
    )]
    pub base_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = position_mint,
        associated_token::authority = vault,
    )]
    pub position_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = counterparty,
    )]
    pub counterparty_base: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = position_mint,
        associated_token::authority = counterparty,
    )]
    pub counterparty_position: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_deploy(ctx: Context<Deploy>, base_out: u64, position_in: u64) -> Result<()> {
    require!(base_out > 0 && position_in > 0, VaultError::InvalidAmount);
    require!(
        ctx.accounts.base_vault.amount >= base_out,
        VaultError::InsufficientLiquidity
    );

    // Counterparty delivers the position first.
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.counterparty_position.to_account_info(),
                to: ctx.accounts.position_vault.to_account_info(),
                authority: ctx.accounts.counterparty.to_account_info(),
            },
        ),
        position_in,
    )?;

    let base_mint = ctx.accounts.base_mint.key();
    let position_mint = ctx.accounts.position_mint.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        VAULT_SEED,
        base_mint.as_ref(),
        position_mint.as_ref(),
        &[ctx.accounts.vault.bump],
    ]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.base_vault.to_account_info(),
                to: ctx.accounts.counterparty_base.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            signer_seeds,
        ),
        base_out,
    )
}
