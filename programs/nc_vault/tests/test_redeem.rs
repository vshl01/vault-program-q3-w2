mod common;

use common::Env;
use solana_signer::Signer;

#[test]
fn redeems_base_while_fully_liquid() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 1_000).unwrap();

    env.redeem(&user, 400).unwrap();

    assert_eq!(env.balance(&env.ata(&user.pubkey(), &env.base_mint)), 400);
    assert_eq!(env.shares_of(&user.pubkey()), 600);
    assert_eq!(env.balance(&env.base_vault()), 600);
}

/// The headline requirement: exiting works with zero liquid base, by paying
/// out the underlying position in kind.
#[test]
fn redeems_in_kind_when_there_is_no_liquidity() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 1_000).unwrap();

    let manager = env.manager.insecure_clone();
    let cp = env.counterparty(500);
    env.deploy(&manager, &cp, 1_000, 500).unwrap();
    assert_eq!(env.balance(&env.base_vault()), 0, "vault should be illiquid");

    env.redeem(&user, 400).unwrap();

    // 400/1000 of the position, and no base because there is none.
    assert_eq!(env.balance(&env.ata(&user.pubkey(), &env.position_mint)), 200);
    assert_eq!(env.balance(&env.ata(&user.pubkey(), &env.base_mint)), 0);
    assert_eq!(env.shares_of(&user.pubkey()), 600);
}

#[test]
fn redeems_a_slice_of_both_when_partly_deployed() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 1_000).unwrap();

    let manager = env.manager.insecure_clone();
    let cp = env.counterparty(400);
    env.deploy(&manager, &cp, 600, 400).unwrap();

    env.redeem(&user, 500).unwrap();

    // Half of 400 base and half of 400 position.
    assert_eq!(env.balance(&env.ata(&user.pubkey(), &env.base_mint)), 200);
    assert_eq!(env.balance(&env.ata(&user.pubkey(), &env.position_mint)), 200);
}

#[test]
fn full_exit_empties_the_vault() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 1_000).unwrap();
    let manager = env.manager.insecure_clone();
    let cp = env.counterparty(500);
    env.deploy(&manager, &cp, 1_000, 500).unwrap();

    env.redeem(&user, 1_000).unwrap();

    assert_eq!(env.shares_of(&user.pubkey()), 0);
    assert_eq!(env.share_supply(), 0);
    assert_eq!(env.balance(&env.position_vault()), 0);
    assert_eq!(env.balance(&env.ata(&user.pubkey(), &env.position_mint)), 500);
}

#[test]
fn two_holders_split_the_position_pro_rata() {
    let mut env = Env::new();
    let big = env.depositor(1_000);
    let small = env.depositor(1_000);
    env.deposit(&big, 750).unwrap();
    env.deposit(&small, 250).unwrap();

    let manager = env.manager.insecure_clone();
    let cp = env.counterparty(400);
    env.deploy(&manager, &cp, 1_000, 400).unwrap();

    env.redeem(&big, 750).unwrap();
    env.redeem(&small, 250).unwrap();

    assert_eq!(env.balance(&env.ata(&big.pubkey(), &env.position_mint)), 300);
    assert_eq!(env.balance(&env.ata(&small.pubkey(), &env.position_mint)), 100);
    assert_eq!(env.balance(&env.position_vault()), 0);
}

#[test]
fn rejects_more_shares_than_owned() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 500).unwrap();

    assert!(env.redeem(&user, 501).is_err());
    assert_eq!(env.shares_of(&user.pubkey()), 500);
}

#[test]
fn rejects_zero_shares() {
    let mut env = Env::new();
    let user = env.depositor(1_000);
    env.deposit(&user, 500).unwrap();

    assert!(env.redeem(&user, 0).is_err());
}

#[test]
fn a_holder_cannot_touch_another_holders_shares() {
    let mut env = Env::new();
    let owner = env.depositor(1_000);
    let other = env.depositor(1_000);
    env.deposit(&owner, 500).unwrap();
    env.deposit(&other, 500).unwrap();

    // `other` can only ever burn from their own share account.
    env.redeem(&other, 500).unwrap();

    assert_eq!(env.shares_of(&owner.pubkey()), 500);
    assert_eq!(env.balance(&env.base_vault()), 500);
}
