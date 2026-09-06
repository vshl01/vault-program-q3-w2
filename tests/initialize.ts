import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { assert } from "chai";

import { VaultProgram } from "../target/types/vault_program";
import idl from "../target/idl/vault_program.json";

describe("vault_program :: initialize", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = new anchor.Program<VaultProgram>(
    idl as VaultProgram,
    provider
  ) as Program<VaultProgram>;

  const payer = (provider.wallet as anchor.Wallet).payer;

  /** Mirrors `seeds = [VAULT_SEED, authority]` in the program. */
  const vaultPda = (authority: PublicKey): [PublicKey, number] =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), authority.toBuffer()],
      program.programId
    );

  const airdrop = async (to: PublicKey, sol: number) => {
    const sig = await provider.connection.requestAirdrop(
      to,
      sol * LAMPORTS_PER_SOL
    );
    const bh = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      { signature: sig, ...bh },
      "confirmed"
    );
  };

  /** A funded keypair, so each test is independent of the others. */
  const fundedAuthority = async (): Promise<Keypair> => {
    const authority = Keypair.generate();
    await airdrop(authority.publicKey, 10);
    return authority;
  };

  const initialize = (authority: Keypair, mint: PublicKey) => {
    const [vault] = vaultPda(authority.publicKey);
    return program.methods
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
      .signers([authority]);
  };

  let mint: PublicKey;

  before(async () => {
    // The provider wallet may be empty on a freshly reset validator.
    if ((await provider.connection.getBalance(payer.publicKey)) < LAMPORTS_PER_SOL) {
      await airdrop(payer.publicKey, 10);
    }

    // 6-decimal mint owned by the payer; the vault never needs mint authority.
    mint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      6
    );
  });

  it("creates the vault PDA with the right authority and bump", async () => {
    const authority = await fundedAuthority();
    const [vault, expectedBump] = vaultPda(authority.publicKey);

    await initialize(authority, mint).rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    assert.isTrue(
      vaultAccount.authority.equals(authority.publicKey),
      "vault.authority should be the signer"
    );
    assert.strictEqual(vaultAccount.bump, expectedBump);

    // The account is owned by our program, not the system program.
    const raw = await provider.connection.getAccountInfo(vault);
    assert.isTrue(raw!.owner.equals(program.programId));
  });

  it("creates an empty vault ATA owned by the vault PDA", async () => {
    const authority = await fundedAuthority();
    const [vault] = vaultPda(authority.publicKey);
    const vaultAta = getAssociatedTokenAddressSync(mint, vault, true);

    await initialize(authority, mint).rpc();

    const ata = await getAccount(provider.connection, vaultAta);
    assert.isTrue(ata.mint.equals(mint));
    assert.isTrue(
      ata.owner.equals(vault),
      "the vault PDA must be the token account authority"
    );
    assert.strictEqual(ata.amount, 0n);
  });

  it("rejects a second initialize for the same authority", async () => {
    const authority = await fundedAuthority();

    await initialize(authority, mint).rpc();

    // Same authority => same vault PDA => `init` must reject it.
    // Note: don't `assert.fail()` inside the try — an AssertionError is itself
    // an Error and would be swallowed by the catch below.
    let error: unknown = null;
    try {
      await initialize(authority, mint).rpc();
    } catch (err) {
      error = err;
    }

    assert.isNotNull(error, "re-initializing an existing vault should have failed");
    assert.match(
      String(error),
      /already in use/i,
      `expected an "already in use" allocation failure, got: ${String(error)}`
    );
  });

  it("derives a distinct vault per authority", async () => {
    const first = await fundedAuthority();
    const second = await fundedAuthority();

    await initialize(first, mint).rpc();
    await initialize(second, mint).rpc();

    const [firstVault] = vaultPda(first.publicKey);
    const [secondVault] = vaultPda(second.publicKey);
    assert.isFalse(firstVault.equals(secondVault));

    const a = await program.account.vault.fetch(firstVault);
    const b = await program.account.vault.fetch(secondVault);
    assert.isTrue(a.authority.equals(first.publicKey));
    assert.isTrue(b.authority.equals(second.publicKey));
  });
});
