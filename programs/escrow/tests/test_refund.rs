mod common;

use common::{escrow_pda, Env, HOUR};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SEED: u64 = 1;
const DEPOSIT: u64 = 400;

fn offer(env: &mut Env) -> Keypair {
    let maker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    let deadline = env.now() + HOUR;
    env.make(&maker, SEED, DEPOSIT, 900, deadline).unwrap();
    maker
}

#[test]
fn returns_the_deposit_after_expiry() {
    let mut env = Env::new();
    let maker = offer(&mut env);
    let maker_ata_a = env.ata(&maker.pubkey(), &env.mint_a);
    assert_eq!(env.token_balance(&maker_ata_a), 600);

    env.warp(HOUR + 1);
    env.refund(&maker, SEED).unwrap();

    assert_eq!(env.token_balance(&maker_ata_a), 1_000);
}

#[test]
fn closes_escrow_and_vault() {
    let mut env = Env::new();
    let maker = offer(&mut env);
    let (escrow, _) = escrow_pda(&maker.pubkey(), SEED);
    let vault = env.ata(&escrow, &env.mint_a);

    env.warp(HOUR + 1);
    env.refund(&maker, SEED).unwrap();

    assert!(env.is_closed(&escrow));
    assert!(env.is_closed(&vault));
}

#[test]
fn rejects_before_the_deadline() {
    let mut env = Env::new();
    let maker = offer(&mut env);

    assert!(env.refund(&maker, SEED).is_err());
    assert_eq!(env.token_balance(&env.ata(&maker.pubkey(), &env.mint_a)), 600);
}

#[test]
fn rejects_a_non_maker() {
    let mut env = Env::new();
    let maker = offer(&mut env);
    let (escrow, _) = escrow_pda(&maker.pubkey(), SEED);

    let attacker = env.funded_keypair();
    env.fund_tokens(&attacker.pubkey(), &env.mint_a.clone(), 0);
    env.warp(HOUR + 1);

    assert!(env.refund_escrow(&attacker, &escrow).is_err());
    assert!(!env.is_closed(&escrow));
}

#[test]
fn cannot_refund_twice() {
    let mut env = Env::new();
    let maker = offer(&mut env);

    env.warp(HOUR + 1);
    env.refund(&maker, SEED).unwrap();

    assert!(env.refund(&maker, SEED).is_err());
}
