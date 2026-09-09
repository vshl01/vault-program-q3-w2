mod common;

use common::{escrow_pda, Env, HOUR};
use solana_signer::Signer;

const SEED: u64 = 1;

#[test]
fn locks_deposit_and_records_terms() {
    let mut env = Env::new();
    let maker = env.funded_keypair();
    let maker_ata_a = env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    let deadline = env.now() + HOUR;
    let (escrow, bump) = escrow_pda(&maker.pubkey(), SEED);

    env.make(&maker, SEED, 400, 900, deadline).unwrap();

    let state = env.escrow_state(&escrow);
    assert_eq!(state.seed, SEED);
    assert_eq!(state.maker, maker.pubkey());
    assert_eq!(state.mint_a, env.mint_a);
    assert_eq!(state.mint_b, env.mint_b);
    assert_eq!(state.receive, 900);
    assert_eq!(state.deadline, deadline);
    assert_eq!(state.bump, bump);

    assert_eq!(env.token_balance(&maker_ata_a), 600);
    assert_eq!(env.token_balance(&env.ata(&escrow, &env.mint_a)), 400);
}

#[test]
fn allows_several_escrows_per_maker() {
    let mut env = Env::new();
    let maker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    let deadline = env.now() + HOUR;

    env.make(&maker, 1, 100, 200, deadline).unwrap();
    env.make(&maker, 2, 300, 400, deadline).unwrap();

    assert_eq!(env.escrow_state(&escrow_pda(&maker.pubkey(), 1).0).receive, 200);
    assert_eq!(env.escrow_state(&escrow_pda(&maker.pubkey(), 2).0).receive, 400);
}

#[test]
fn rejects_reusing_a_seed() {
    let mut env = Env::new();
    let maker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    let deadline = env.now() + HOUR;

    env.make(&maker, SEED, 100, 200, deadline).unwrap();

    assert!(env.make(&maker, SEED, 100, 200, deadline).is_err());
}

#[test]
fn rejects_zero_amounts() {
    let mut env = Env::new();
    let maker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    let deadline = env.now() + HOUR;

    assert!(env.make(&maker, 1, 0, 200, deadline).is_err());
    assert!(env.make(&maker, 2, 100, 0, deadline).is_err());
}

#[test]
fn rejects_deadline_in_the_past() {
    let mut env = Env::new();
    let maker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 1_000);
    let past = env.now() - 1;

    assert!(env.make(&maker, SEED, 100, 200, past).is_err());
}

#[test]
fn rejects_deposit_above_balance() {
    let mut env = Env::new();
    let maker = env.funded_keypair();
    env.fund_tokens(&maker.pubkey(), &env.mint_a.clone(), 100);
    let deadline = env.now() + HOUR;

    assert!(env.make(&maker, SEED, 101, 200, deadline).is_err());
}
