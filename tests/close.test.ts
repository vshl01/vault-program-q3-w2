import { beforeEach, describe, expect, test } from "vitest";
import { Keypair, PublicKey } from "@solana/web3.js";
import { FailedTransactionMetadata } from "litesvm";

import { Env, vaultPda } from "./env";

const isGone = (env: Env, address: PublicKey) => {
  const account = env.svm.getAccount(address);
  return account === null || account.data.length === 0;
};

describe("close", () => {
  let env: Env;
  let authority: Keypair;
  let vault: PublicKey;
  let userAta: PublicKey;

  beforeEach(async () => {
    env = new Env();
    authority = env.fundedKeypair();
    userAta = env.fundTokens(authority.publicKey, 1_000);
    env.expectOk(await env.initialize(authority));
    vault = vaultPda(authority.publicKey)[0];
  });

  test("closes the vault and returns rent to the authority", async () => {
    const before = env.svm.getBalance(authority.publicKey)!;

    env.expectOk(await env.close(authority));

    expect(isGone(env, vault)).toBe(true);
    expect(env.svm.getBalance(authority.publicKey)!).toBeGreaterThan(before);
  });

  test("closes the vault token account", async () => {
    env.expectOk(await env.close(authority));

    expect(isGone(env, env.ata(vault))).toBe(true);
  });

  test("sweeps remaining tokens back to the authority", async () => {
    env.expectOk(await env.deposit(authority, 700));
    expect(env.tokenBalance(userAta)).toBe(300n);

    env.expectOk(await env.close(authority));

    expect(env.tokenBalance(userAta)).toBe(1_000n);
  });

  test("rejects closing someone else's vault", async () => {
    const attacker = env.fundedKeypair();
    env.fundTokens(attacker.publicKey, 0);

    const res = await env.closeVault(attacker, vault);

    expect(res).toBeInstanceOf(FailedTransactionMetadata);
    expect(env.vaultState(vault).authority.equals(authority.publicKey)).toBe(true);
  });
});
