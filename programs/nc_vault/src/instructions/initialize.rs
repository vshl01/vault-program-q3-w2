use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{constants::*, error::VaultError, state::Vault};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,

    pub base_mint: Box<Account<'info, Mint>>,
    pub position_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = manager,
        space = 8 + Vault::INIT_SPACE,
        seeds = [VAULT_SEED, base_mint.key().as_ref(), position_mint.key().as_ref()],
        bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        payer = manager,
        seeds = [SHARE_SEED, vault.key().as_ref()],
        bump,
        mint::decimals = base_mint.decimals,
        mint::authority = vault,
    )]
    pub share_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = manager,
        associated_token::mint = base_mint,
        associated_token::authority = vault,
    )]
    pub base_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = manager,
        associated_token::mint = position_mint,
        associated_token::authority = vault,
    )]
    pub position_vault: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>) -> Result<()> {
    require_keys_neq!(
        ctx.accounts.base_mint.key(),
        ctx.accounts.position_mint.key(),
        VaultError::SameMint
    );

    ctx.accounts.vault.set_inner(Vault {
        manager: ctx.accounts.manager.key(),
        base_mint: ctx.accounts.base_mint.key(),
        position_mint: ctx.accounts.position_mint.key(),
        share_mint: ctx.accounts.share_mint.key(),
        bump: ctx.bumps.vault,
    });
    Ok(())
}
