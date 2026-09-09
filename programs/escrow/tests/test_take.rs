mod common;

use common::{escrow_pda, Env, HOUR};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SEED: u64 = 1;
const DEPOSIT: u64 = 400;
const PRICE: u64 = 900;

/// Maker offering DEPOSIT of mint_a for PRICE of mint_b, plus a funded taker.
fn offer(env: &mut Env) -> (Keypair, Keypair) {
    let maker = env.funded_keypair();
    let taker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    env.fund_tokens(&taker.pubkey(), &env.mint_b.clone(), 1_000);
    let deadline = env.now() + HOUR;
    env.make(&maker, SEED, DEPOSIT, PRICE, deadline).unwrap();
    (maker, taker)
}

#[test]
fn swaps_both_sides() {
    let mut env = Env::new();
    let (maker, taker) = offer(&mut env);

    env.take(&taker, &maker.pubkey(), SEED, PRICE).unwrap();

    assert_eq!(env.token_balance(&env.ata(&taker.pubkey(), &env.mint_a)), DEPOSIT);
    assert_eq!(env.token_balance(&env.ata(&maker.pubkey(), &env.mint_b)), PRICE);
    assert_eq!(env.token_balance(&env.ata(&taker.pubkey(), &env.mint_b)), 1_000 - PRICE);
    assert_eq!(env.token_balance(&env.ata(&maker.pubkey(), &env.mint_a)), 1_000 - DEPOSIT);
}

#[test]
fn closes_escrow_and_vault() {
    let mut env = Env::new();
    let (maker, taker) = offer(&mut env);
    let (escrow, _) = escrow_pda(&maker.pubkey(), SEED);
    let vault = env.ata(&escrow, &env.mint_a);

    env.take(&taker, &maker.pubkey(), SEED, PRICE).unwrap();

    assert!(env.is_closed(&escrow));
    assert!(env.is_closed(&vault));
}

#[test]
fn returns_rent_to_maker() {
    let mut env = Env::new();
    let (maker, taker) = offer(&mut env);
    let before = env.svm.get_balance(&maker.pubkey()).unwrap();

    env.take(&taker, &maker.pubkey(), SEED, PRICE).unwrap();

    assert!(env.svm.get_balance(&maker.pubkey()).unwrap() > before);
}

#[test]
fn rejects_after_the_deadline() {
    let mut env = Env::new();
    let (maker, taker) = offer(&mut env);

    env.warp(HOUR + 1);

    assert!(env.take(&taker, &maker.pubkey(), SEED, PRICE).is_err());
}

#[test]
fn rejects_a_stale_price() {
    let mut env = Env::new();
    let (maker, taker) = offer(&mut env);
    env.update(&maker, SEED, PRICE * 5).unwrap();

    // The taker still believes the original price.
    assert!(env.take(&taker, &maker.pubkey(), SEED, PRICE).is_err());
}

#[test]
fn rejects_a_taker_who_cannot_pay() {
    let mut env = Env::new();
    let maker = env.funded_keypair();
    let taker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    env.fund_tokens(&taker.pubkey(), &env.mint_b.clone(), PRICE - 1);
    let deadline = env.now() + HOUR;
    env.make(&maker, SEED, DEPOSIT, PRICE, deadline).unwrap();

    assert!(env.take(&taker, &maker.pubkey(), SEED, PRICE).is_err());
}

#[test]
fn cannot_be_taken_twice() {
    let mut env = Env::new();
    let (maker, taker) = offer(&mut env);

    env.take(&taker, &maker.pubkey(), SEED, PRICE).unwrap();

    assert!(env.take(&taker, &maker.pubkey(), SEED, PRICE).is_err());
}
