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

  beforeEach(async () => {
    env = new Env();
    authority = env.fundedKeypair();
    env.expectOk(await env.initialize(authority));
    vault = vaultPda(authority.publicKey)[0];
  });

  test("closes the vault and returns rent to the authority", async () => {
    const before = env.svm.getBalance(authority.publicKey)!;

    env.expectOk(await env.close(authority));

    expect(isGone(env, vault)).toBe(true);
    expect(env.svm.getBalance(authority.publicKey)!).toBeGreaterThan(before);
  });

  test("closes the empty vault token account", async () => {
    env.expectOk(await env.close(authority));

    expect(isGone(env, env.ata(vault))).toBe(true);
  });

  test("rejects closing someone else's vault", async () => {
    const attacker = env.fundedKeypair();

    const res = await env.closeVault(attacker, vault);

    expect(res).toBeInstanceOf(FailedTransactionMetadata);
    expect(env.vaultState(vault).authority.equals(authority.publicKey)).toBe(true);
  });
});
