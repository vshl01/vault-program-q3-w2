mod common;

use common::{escrow_pda, Env, HOUR};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SEED: u64 = 1;
const DEPOSIT: u64 = 400;
const PRICE: u64 = 900;

fn offer(env: &mut Env) -> Keypair {
    let maker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    let deadline = env.now() + HOUR;
    env.make(&maker, SEED, DEPOSIT, PRICE, deadline).unwrap();
    maker
}

#[test]
fn changes_the_asking_price() {
    let mut env = Env::new();
    let maker = offer(&mut env);
    let (escrow, _) = escrow_pda(&maker.pubkey(), SEED);

    env.update(&maker, SEED, 1_500).unwrap();

    assert_eq!(env.escrow_state(&escrow).receive, 1_500);
}

#[test]
fn leaves_the_deposit_untouched() {
    let mut env = Env::new();
    let maker = offer(&mut env);
    let (escrow, _) = escrow_pda(&maker.pubkey(), SEED);

    env.update(&maker, SEED, 1_500).unwrap();

    assert_eq!(env.token_balance(&env.ata(&escrow, &env.mint_a)), DEPOSIT);
}

#[test]
fn a_taker_pays_the_new_price() {
    let mut env = Env::new();
    let maker = offer(&mut env);
    let taker = env.funded_keypair();
    env.fund_tokens(&taker.pubkey(), &env.mint_b.clone(), 2_000);

    env.update(&maker, SEED, 1_500).unwrap();
    env.take(&taker, &maker.pubkey(), SEED, 1_500).unwrap();

    assert_eq!(env.token_balance(&env.ata(&maker.pubkey(), &env.mint_b)), 1_500);
}

#[test]
fn rejects_zero() {
    let mut env = Env::new();
    let maker = offer(&mut env);

    assert!(env.update(&maker, SEED, 0).is_err());
}

#[test]
fn rejects_a_non_maker() {
    let mut env = Env::new();
    let maker = offer(&mut env);
    let (escrow, _) = escrow_pda(&maker.pubkey(), SEED);
    let attacker = env.funded_keypair();

    assert!(env.update_escrow(&attacker, &escrow, 1).is_err());
    assert_eq!(env.escrow_state(&escrow).receive, PRICE);
}

#[test]
fn rejects_after_the_deadline() {
    let mut env = Env::new();
    let maker = offer(&mut env);

    env.warp(HOUR + 1);

    assert!(env.update(&maker, SEED, 1_500).is_err());
}
