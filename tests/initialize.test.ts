import { beforeEach, describe, expect, test } from "vitest";
import * as anchor from "@coral-xyz/anchor";
import { FailedTransactionMetadata, LiteSVM } from "litesvm";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  AccountLayout,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createInitializeMint2Instruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

import { VaultProgram } from "../target/types/vault_program";
import idl from "../target/idl/vault_program.json";

const PROGRAM_SO = "target/deploy/vault_program.so";
const PROGRAM_ID = new PublicKey(idl.address);

// LiteSVM replaces the RPC layer entirely, so this Connection is never dialled.
// The Program object is used only to build instructions and decode account data.
const program = new anchor.Program<VaultProgram>(idl as VaultProgram, {
  connection: new Connection("http://127.0.0.1:8899"),
} as anchor.Provider);

/** Mirrors `seeds = [VAULT_SEED, authority]` in the program. */
const vaultPda = (authority: PublicKey): [PublicKey, number] =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), authority.toBuffer()],
    PROGRAM_ID
  );

describe("vault_program :: initialize", () => {
  let svm: LiteSVM;
  let payer: Keypair;
  let mint: PublicKey;

  /** Signs and submits a transaction to the in-process VM. signers[0] pays. */
  const send = (ixs: TransactionInstruction[], signers: Keypair[]) => {
    const tx = new Transaction();
    tx.recentBlockhash = svm.latestBlockhash();
    tx.feePayer = signers[0].publicKey;
    tx.add(...ixs);
    tx.sign(...signers);
    return svm.sendTransaction(tx);
  };

  /** Fails loudly with program logs instead of a bare boolean. */
  const expectOk = (res: ReturnType<typeof send>) => {
    if (res instanceof FailedTransactionMetadata) {
      throw new Error(
        `transaction failed: ${res.err()}\n${res.meta().logs().join("\n")}`
      );
    }
    return res;
  };

  const fundedKeypair = (sol = 10): Keypair => {
    const kp = Keypair.generate();
    svm.airdrop(kp.publicKey, BigInt(sol * LAMPORTS_PER_SOL));
    return kp;
  };

  const createMint = (authority: Keypair): PublicKey => {
    const mintKp = Keypair.generate();
    expectOk(
      send(
        [
          SystemProgram.createAccount({
            fromPubkey: authority.publicKey,
            newAccountPubkey: mintKp.publicKey,
            lamports: Number(svm.minimumBalanceForRentExemption(BigInt(MINT_SIZE))),
            space: MINT_SIZE,
            programId: TOKEN_PROGRAM_ID,
          }),
          createInitializeMint2Instruction(
            mintKp.publicKey,
            6,
            authority.publicKey,
            null
          ),
        ],
        [authority, mintKp]
      )
    );
    return mintKp.publicKey;
  };

  const initialize = async (authority: Keypair, mint: PublicKey) => {
    const [vault] = vaultPda(authority.publicKey);
    const ix = await program.methods
      .initialize()
      .accountsPartial({
        authority: authority.publicKey,
        vault,
        systemProgram: SystemProgram.programId,
        mint,
        vaultTokenAccount: getAssociatedTokenAddressSync(mint, vault, true),
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .instruction();
    return send([ix], [authority]);
  };

  // A fresh VM per test: no shared state, and it costs microseconds.
  beforeEach(() => {
    svm = new LiteSVM();
    svm.addProgramFromFile(PROGRAM_ID, PROGRAM_SO);
    payer = fundedKeypair();
    mint = createMint(payer);
  });

  test("creates the vault PDA with the right authority and bump", async () => {
    const authority = fundedKeypair();
    const [vault, expectedBump] = vaultPda(authority.publicKey);

    expectOk(await initialize(authority, mint));

    const raw = svm.getAccount(vault);
    expect(raw, "vault account was not created").not.toBeNull();
    expect(raw!.owner.equals(PROGRAM_ID)).toBe(true);

    const decoded = program.coder.accounts.decode(
      "vault",
      Buffer.from(raw!.data)
    );
    expect(decoded.authority.equals(authority.publicKey)).toBe(true);
    expect(decoded.bump).toBe(expectedBump);
  });

  test("creates an empty vault ATA owned by the vault PDA", async () => {
    const authority = fundedKeypair();
    const [vault] = vaultPda(authority.publicKey);
    const vaultAta = getAssociatedTokenAddressSync(mint, vault, true);

    expectOk(await initialize(authority, mint));

    const raw = svm.getAccount(vaultAta);
    expect(raw, "vault ATA was not created").not.toBeNull();
    expect(raw!.owner.equals(TOKEN_PROGRAM_ID)).toBe(true);

    const token = AccountLayout.decode(Buffer.from(raw!.data));
    expect(token.mint.equals(mint)).toBe(true);
    expect(token.owner.equals(vault)).toBe(true);
    expect(token.amount).toBe(0n);
  });

  test("rejects a second initialize for the same authority", async () => {
    const authority = fundedKeypair();

    expectOk(await initialize(authority, mint));

    // Same authority => same vault PDA => `init` must reject it.
    svm.expireBlockhash();
    const res = await initialize(authority, mint);

    expect(res).toBeInstanceOf(FailedTransactionMetadata);
    expect((res as FailedTransactionMetadata).meta().logs().join("\n")).toMatch(
      /already in use/i
    );
  });

  test("derives a distinct vault per authority", async () => {
    const first = fundedKeypair();
    const second = fundedKeypair();

    expectOk(await initialize(first, mint));
    expectOk(await initialize(second, mint));

    const [firstVault] = vaultPda(first.publicKey);
    const [secondVault] = vaultPda(second.publicKey);
    expect(firstVault.equals(secondVault)).toBe(false);

    const a = program.coder.accounts.decode(
      "vault",
      Buffer.from(svm.getAccount(firstVault)!.data)
    );
    const b = program.coder.accounts.decode(
      "vault",
      Buffer.from(svm.getAccount(secondVault)!.data)
    );
    expect(a.authority.equals(first.publicKey)).toBe(true);
    expect(b.authority.equals(second.publicKey)).toBe(true);
  });
});
