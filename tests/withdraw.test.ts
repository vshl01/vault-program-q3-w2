import { beforeEach, describe, expect, test } from "vitest";
import { Keypair, PublicKey } from "@solana/web3.js";
import { FailedTransactionMetadata } from "litesvm";

import { Env, vaultPda } from "./env";

describe("withdraw", () => {
  let env: Env;
  let authority: Keypair;
  let userAta: PublicKey;
  let vaultAta: PublicKey;

  beforeEach(async () => {
    env = new Env();
    authority = env.fundedKeypair();
    userAta = env.fundTokens(authority.publicKey, 1_000);
    env.expectOk(await env.initialize(authority));
    vaultAta = env.ata(vaultPda(authority.publicKey)[0]);
    env.expectOk(await env.deposit(authority, 500));
  });

  test("returns tokens to the authority", async () => {
    env.expectOk(await env.withdraw(authority, 200));

    expect(env.tokenBalance(userAta)).toBe(700n);
    expect(env.tokenBalance(vaultAta)).toBe(300n);
  });

  test("drains the full balance", async () => {
    env.expectOk(await env.withdraw(authority, 500));

    expect(env.tokenBalance(userAta)).toBe(1_000n);
    expect(env.tokenBalance(vaultAta)).toBe(0n);
  });

  test("rejects an amount above the vault balance", async () => {
    const res = await env.withdraw(authority, 501);

    expect(res).toBeInstanceOf(FailedTransactionMetadata);
    expect(env.tokenBalance(vaultAta)).toBe(500n);
  });

  test("rejects a withdraw from someone else's vault", async () => {
    const attacker = env.fundedKeypair();
    env.fundTokens(attacker.publicKey, 0);

    const res = await env.withdrawFrom(
      attacker,
      vaultPda(authority.publicKey)[0],
      500
    );

    expect(res).toBeInstanceOf(FailedTransactionMetadata);
    expect(env.tokenBalance(vaultAta)).toBe(500n);
  });
});
