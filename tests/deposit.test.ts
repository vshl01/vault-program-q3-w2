import { beforeEach, describe, expect, test } from "vitest";
import { Keypair, PublicKey } from "@solana/web3.js";
import { FailedTransactionMetadata } from "litesvm";

import { Env, vaultPda } from "./env";

describe("deposit", () => {
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
  });

  test("moves tokens from the user to the vault", async () => {
    env.expectOk(await env.deposit(authority, 400));

    expect(env.tokenBalance(userAta)).toBe(600n);
    expect(env.tokenBalance(vaultAta)).toBe(400n);
  });

  test("accumulates across deposits", async () => {
    env.expectOk(await env.deposit(authority, 300));
    env.expectOk(await env.deposit(authority, 250));

    expect(env.tokenBalance(userAta)).toBe(450n);
    expect(env.tokenBalance(vaultAta)).toBe(550n);
  });

  test("treats a zero amount as a no-op", async () => {
    env.expectOk(await env.deposit(authority, 0));

    expect(env.tokenBalance(userAta)).toBe(1_000n);
    expect(env.tokenBalance(vaultAta)).toBe(0n);
  });

  test("rejects an amount above the user balance", async () => {
    const res = await env.deposit(authority, 1_001);

    expect(res).toBeInstanceOf(FailedTransactionMetadata);
    expect(env.tokenBalance(userAta)).toBe(1_000n);
  });

  test("rejects a deposit when no vault exists", async () => {
    const stranger = env.fundedKeypair();
    env.fundTokens(stranger.publicKey, 500);

    const res = await env.deposit(stranger, 100);

    expect(res).toBeInstanceOf(FailedTransactionMetadata);
  });
});
