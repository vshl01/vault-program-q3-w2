mod common;

use anchor_spl::associated_token::get_associated_token_address;
use common::{vault_pda, Env};
use solana_signer::Signer;

/// Vault holding `deposited` tokens, owned by the returned authority.
fn vault_with_tokens(env: &mut Env, deposited: u64) -> (solana_keypair::Keypair, u64) {
    let authority = env.funded_keypair();
    env.fund_tokens(&authority.pubkey(), 1_000);
    env.initialize(&authority).unwrap();
    env.deposit(&authority, deposited).unwrap();
    (authority, 1_000 - deposited)
}

#[test]
fn returns_tokens_to_authority() {
    let mut env = Env::new();
    let (authority, user_remaining) = vault_with_tokens(&mut env, 500);
    let user_ata = env.ata(&authority.pubkey());
    let vault_ata = get_associated_token_address(&vault_pda(&authority.pubkey()).0, &env.mint);

    env.withdraw(&authority, 200).unwrap();

    assert_eq!(env.token_balance(&user_ata), user_remaining + 200);
    assert_eq!(env.token_balance(&vault_ata), 300);
}

#[test]
fn drains_full_balance() {
    let mut env = Env::new();
    let (authority, _) = vault_with_tokens(&mut env, 500);
    let vault_ata = get_associated_token_address(&vault_pda(&authority.pubkey()).0, &env.mint);

    env.withdraw(&authority, 500).unwrap();

    assert_eq!(env.token_balance(&vault_ata), 0);
    assert_eq!(env.token_balance(&env.ata(&authority.pubkey())), 1_000);
}

#[test]
fn rejects_amount_above_vault_balance() {
    let mut env = Env::new();
    let (authority, _) = vault_with_tokens(&mut env, 500);
    let vault_ata = get_associated_token_address(&vault_pda(&authority.pubkey()).0, &env.mint);

    assert!(env.withdraw(&authority, 501).is_err());
    assert_eq!(env.token_balance(&vault_ata), 500);
}

#[test]
fn rejects_withdraw_from_another_vault() {
    let mut env = Env::new();
    let (victim, _) = vault_with_tokens(&mut env, 500);
    let (victim_vault, _) = vault_pda(&victim.pubkey());

    let attacker = env.funded_keypair();
    env.fund_tokens(&attacker.pubkey(), 0);

    assert!(env.withdraw_from(&attacker, &victim_vault, 500).is_err());
    assert_eq!(
        env.token_balance(&get_associated_token_address(&victim_vault, &env.mint)),
        500
    );
}
