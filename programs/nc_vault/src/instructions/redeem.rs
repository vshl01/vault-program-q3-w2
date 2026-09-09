use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, Mint, Token, TokenAccount, Transfer},
};

use crate::{constants::*, error::VaultError, state::Vault};

/// Exit. Burns shares and pays out a pro-rata slice of everything the vault
/// holds — base if there is any, and the position in kind for the rest.
/// Never depends on liquidity, so a holder can always leave.
#[derive(Accounts)]
pub struct Redeem<'info> {
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
        mut,
        associated_token::mint = position_mint,
        associated_token::authority = vault,
    )]
    pub position_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = base_mint,
        associated_token::authority = user,
    )]
    pub user_base: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = position_mint,
        associated_token::authority = user,
    )]
    pub user_position: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = share_mint,
        associated_token::authority = user,
    )]
    pub user_shares: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_redeem(ctx: Context<Redeem>, shares: u64) -> Result<()> {
    require!(shares > 0, VaultError::InvalidAmount);
    require!(
        ctx.accounts.user_shares.amount >= shares,
        VaultError::InsufficientShares
    );

    // Pro-rata against the supply before the burn.
    let supply = ctx.accounts.share_mint.supply as u128;
    let shares_u128 = shares as u128;
    let base_out = (ctx.accounts.base_vault.amount as u128 * shares_u128 / supply) as u64;
    let position_out = (ctx.accounts.position_vault.amount as u128 * shares_u128 / supply) as u64;

    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Burn {
                mint: ctx.accounts.share_mint.to_account_info(),
                from: ctx.accounts.user_shares.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        shares,
    )?;

    let base_mint = ctx.accounts.base_mint.key();
    let position_mint = ctx.accounts.position_mint.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        VAULT_SEED,
        base_mint.as_ref(),
        position_mint.as_ref(),
        &[ctx.accounts.vault.bump],
    ]];

    if base_out > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.base_vault.to_account_info(),
                    to: ctx.accounts.user_base.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signer_seeds,
            ),
            base_out,
        )?;
    }

    if position_out > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.position_vault.to_account_info(),
                    to: ctx.accounts.user_position.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signer_seeds,
            ),
            position_out,
        )?;
    }

    Ok(())
}
