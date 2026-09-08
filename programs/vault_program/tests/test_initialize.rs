mod common;

use anchor_spl::{associated_token::get_associated_token_address, token::spl_token};
use anchor_lang::solana_program::program_pack::Pack;
use common::{vault_pda, Env};
use solana_signer::Signer;

#[test]
fn creates_vault_with_authority_and_bump() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    let (vault, bump) = vault_pda(&authority.pubkey());

    env.initialize(&authority).unwrap();

    let state = env.vault_state(&vault);
    assert_eq!(state.authority, authority.pubkey());
    assert_eq!(state.bump, bump);
    assert_eq!(env.svm.get_account(&vault).unwrap().owner, vault_program::ID);
}

#[test]
fn creates_empty_vault_token_account() {
    let mut env = Env::new();
    let authority = env.funded_keypair();
    let (vault, _) = vault_pda(&authority.pubkey());

    env.initialize(&authority).unwrap();

    let ata = get_associated_token_address(&vault, &env.mint);
    let account = env.svm.get_account(&ata).expect("vault ATA missing");
    assert_eq!(account.owner, spl_token::ID);

    let token = spl_token::state::Account::unpack(&account.data).unwrap();
    assert_eq!(token.mint, env.mint);
    assert_eq!(token.owner, vault);
    assert_eq!(token.amount, 0);
}

#[test]
fn rejects_second_initialize() {
    let mut env = Env::new();
    let authority = env.funded_keypair();

    env.initialize(&authority).unwrap();

    assert!(env.initialize(&authority).is_err());
}

#[test]
fn derives_distinct_vault_per_authority() {
    let mut env = Env::new();
    let first = env.funded_keypair();
    let second = env.funded_keypair();

    env.initialize(&first).unwrap();
    env.initialize(&second).unwrap();

    let (first_vault, _) = vault_pda(&first.pubkey());
    let (second_vault, _) = vault_pda(&second.pubkey());
    assert_ne!(first_vault, second_vault);
    assert_eq!(env.vault_state(&first_vault).authority, first.pubkey());
    assert_eq!(env.vault_state(&second_vault).authority, second.pubkey());
}
