mod common;

use common::Env;
use solana_signer::Signer;

#[test]
fn initialize_wires_up_the_vault() {
    let env = Env::new();

    let state = env.vault_state();
    assert_eq!(state.manager, env.manager.pubkey());
    assert_eq!(state.base_mint, env.base_mint);
    assert_eq!(state.position_mint, env.position_mint);
    assert_eq!(state.share_mint, env.share_mint());
    assert_eq!(env.share_supply(), 0);
    assert_eq!(env.balance(&env.base_vault()), 0);
    assert_eq!(env.balance(&env.position_vault()), 0);
}

#[test]
fn first_deposit_mints_shares_one_to_one() {
    let mut env = Env::new();
    let user = env.depositor(1_000);

    env.deposit(&user, 400).unwrap();

    assert_eq!(env.shares_of(&user.pubkey()), 400);
    assert_eq!(env.balance(&env.base_vault()), 400);
    assert_eq!(env.balance(&env.ata(&user.pubkey(), &env.base_mint)), 600);
}

#[test]
fn later_deposits_are_pro_rata() {
    let mut env = Env::new();
    let first = env.depositor(1_000);
    let second = env.depositor(1_000);

    env.deposit(&first, 400).unwrap();
    env.deposit(&second, 200).unwrap();

    assert_eq!(env.shares_of(&first.pubkey()), 400);
    assert_eq!(env.shares_of(&second.pubkey()), 200);
    assert_eq!(env.share_supply(), 600);
}

#[test]
fn deposit_rejects_zero() {
    let mut env = Env::new();
    let user = env.depositor(1_000);

    assert!(env.deposit(&user, 0).is_err());
}

#[test]
fn deploy_swaps_base_for_position() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 1_000).unwrap();
    let manager = env.manager.insecure_clone();
    let cp = env.counterparty(500);

    env.deploy(&manager, &cp, 1_000, 500).unwrap();

    assert_eq!(env.balance(&env.base_vault()), 0);
    assert_eq!(env.balance(&env.position_vault()), 500);
    assert_eq!(env.balance(&env.ata(&cp.pubkey(), &env.base_mint)), 1_000);
}

#[test]
fn deploy_rejects_a_non_manager() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 1_000).unwrap();
    let impostor = env.funded_keypair();
    let cp = env.counterparty(500);

    assert!(env.deploy(&impostor, &cp, 1_000, 500).is_err());
    assert_eq!(env.balance(&env.base_vault()), 1_000);
}

#[test]
fn deploy_rejects_more_base_than_held() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 500).unwrap();
    let manager = env.manager.insecure_clone();
    let cp = env.counterparty(500);

    assert!(env.deploy(&manager, &cp, 501, 500).is_err());
}

#[test]
fn deposit_is_refused_once_deployed() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 500).unwrap();
    let manager = env.manager.insecure_clone();
    let cp = env.counterparty(500);
    env.deploy(&manager, &cp, 500, 500).unwrap();

    assert!(env.deposit(&user, 100).is_err());
}
