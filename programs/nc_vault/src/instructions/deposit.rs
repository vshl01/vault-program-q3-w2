use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{constants::*, error::VaultError, state::Vault};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        has_one = base_mint,
        has_one = position_mint,
        has_one = share_mint,
        seeds = [VAULT_SEED, base_mint.key().as_ref(), position_mint.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub base_mint: Box<Account<'info, Mint>>,
    pub position_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub share_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = vault,
    )]
    pub base_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        associated_token::mint = position_mint,
        associated_token::authority = vault,
    )]
    pub position_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = user,
    )]
    pub user_base: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = share_mint,
        associated_token::authority = user,
    )]
    pub user_shares: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);
    // Pricing a deposit against a live position would need an oracle, so
    // deposits are only accepted while the vault is fully liquid.
    require!(
        ctx.accounts.position_vault.amount == 0,
        VaultError::VaultDeployed
    );

    let supply = ctx.accounts.share_mint.supply;
    let pooled = ctx.accounts.base_vault.amount;
    let shares = if supply == 0 {
        amount
    } else {
        (amount as u128 * supply as u128 / pooled as u128) as u64
    };
    require!(shares > 0, VaultError::InvalidAmount);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_base.to_account_info(),
                to: ctx.accounts.base_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    let base_mint = ctx.accounts.base_mint.key();
    let position_mint = ctx.accounts.position_mint.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        VAULT_SEED,
        base_mint.as_ref(),
        position_mint.as_ref(),
        &[ctx.accounts.vault.bump],
    ]];

    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintTo {
                mint: ctx.accounts.share_mint.to_account_info(),
                to: ctx.accounts.user_shares.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            signer_seeds,
        ),
        shares,
    )
}
