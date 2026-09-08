mod common;

use anchor_spl::associated_token::get_associated_token_address;
use common::{vault_pda, Env};
use solana_signer::Signer;

#[test]
fn moves_tokens_from_user_to_vault() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    let user_ata = env.fund_tokens(&authority.pubkey(), 1_000);
    env.initialize(&authority).unwrap();
    let vault_ata = get_associated_token_address(&vault_pda(&authority.pubkey()).0, &env.mint);

    env.deposit(&authority, 400).unwrap();

    assert_eq!(env.token_balance(&user_ata), 600);
    assert_eq!(env.token_balance(&vault_ata), 400);
}

#[test]
fn accumulates_across_deposits() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    let user_ata = env.fund_tokens(&authority.pubkey(), 1_000);
    env.initialize(&authority).unwrap();
    let vault_ata = get_associated_token_address(&vault_pda(&authority.pubkey()).0, &env.mint);

    env.deposit(&authority, 300).unwrap();
    env.deposit(&authority, 250).unwrap();

    assert_eq!(env.token_balance(&user_ata), 450);
    assert_eq!(env.token_balance(&vault_ata), 550);
}

#[test]
fn zero_amount_is_a_noop() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    let user_ata = env.fund_tokens(&authority.pubkey(), 1_000);
    env.initialize(&authority).unwrap();

    env.deposit(&authority, 0).unwrap();

    assert_eq!(env.token_balance(&user_ata), 1_000);
}

#[test]
fn rejects_amount_above_balance() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    let user_ata = env.fund_tokens(&authority.pubkey(), 100);
    env.initialize(&authority).unwrap();

    assert!(env.deposit(&authority, 101).is_err());
    assert_eq!(env.token_balance(&user_ata), 100);
}

#[test]
fn rejects_deposit_without_a_vault() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    env.fund_tokens(&authority.pubkey(), 1_000);

    assert!(env.deposit(&authority, 100).is_err());
}
