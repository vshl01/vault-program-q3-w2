import { beforeEach, describe, expect, test } from "vitest";
import { AccountLayout, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Keypair } from "@solana/web3.js";

import { Env, PROGRAM_ID, vaultPda } from "./env";

describe("initialize", () => {
  let env: Env;
  let authority: Keypair;

  beforeEach(() => {
    env = new Env();
    authority = env.fundedKeypair();
  });

  test("creates the vault with the right authority and bump", async () => {
    const [vault, bump] = vaultPda(authority.publicKey);

    env.expectOk(await env.initialize(authority));

    const state = env.vaultState(vault);
    expect(state.authority.equals(authority.publicKey)).toBe(true);
    expect(state.bump).toBe(bump);
    expect(env.svm.getAccount(vault)!.owner.equals(PROGRAM_ID)).toBe(true);
  });

  test("creates an empty vault token account owned by the vault", async () => {
    const [vault] = vaultPda(authority.publicKey);

    env.expectOk(await env.initialize(authority));

    const account = env.svm.getAccount(env.ata(vault));
    expect(account, "vault ATA missing").not.toBeNull();
    expect(account!.owner.equals(TOKEN_PROGRAM_ID)).toBe(true);

    const token = AccountLayout.decode(Buffer.from(account!.data));
    expect(token.mint.equals(env.mint)).toBe(true);
    expect(token.owner.equals(vault)).toBe(true);
    expect(token.amount).toBe(0n);
  });

  test("rejects a second initialize", async () => {
    env.expectOk(await env.initialize(authority));

    const res = await env.initialize(authority);
    expect(res.toString()).toMatch(/already in use/i);
  });

  test("derives a distinct vault per authority", async () => {
    const other = env.fundedKeypair();

    env.expectOk(await env.initialize(authority));
    env.expectOk(await env.initialize(other));

    const [first] = vaultPda(authority.publicKey);
    const [second] = vaultPda(other.publicKey);
    expect(first.equals(second)).toBe(false);
    expect(env.vaultState(first).authority.equals(authority.publicKey)).toBe(true);
    expect(env.vaultState(second).authority.equals(other.publicKey)).toBe(true);
  });
});
