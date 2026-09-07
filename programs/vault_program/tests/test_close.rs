mod common;

use anchor_spl::associated_token::get_associated_token_address;
use common::{vault_pda, Env};
use solana_signer::Signer;

#[test]
fn closes_vault_and_returns_rent() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    env.initialize(&authority).unwrap();
    let (vault, _) = vault_pda(&authority.pubkey());
    let before = env.svm.get_balance(&authority.pubkey()).unwrap();

    env.close(&authority).unwrap();

    assert!(env.svm.get_account(&vault).is_none_or(|a| a.data.is_empty()));
    assert!(env.svm.get_balance(&authority.pubkey()).unwrap() > before);
}

#[test]
fn closes_empty_vault_token_account() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    env.initialize(&authority).unwrap();
    let vault_ata = get_associated_token_address(&vault_pda(&authority.pubkey()).0, &env.mint);

    env.close(&authority).unwrap();

    assert!(env.svm.get_account(&vault_ata).is_none_or(|a| a.data.is_empty()));
}

#[test]
fn rejects_close_of_another_vault() {
    let mut env = Env::new();
    let victim = env.funded_keypair();
    env.initialize(&victim).unwrap();
    let (victim_vault, _) = vault_pda(&victim.pubkey());

    let attacker = env.funded_keypair();

    assert!(env.close_vault(&attacker, &victim_vault).is_err());
    assert_eq!(env.vault_state(&victim_vault).authority, victim.pubkey());
}
