import * as anchor from "@coral-xyz/anchor";
import { FailedTransactionMetadata, LiteSVM } from "litesvm";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  AccountLayout,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  createInitializeMint2Instruction,
  createMintToInstruction,
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

export const PROGRAM_ID = new PublicKey(idl.address);
const PROGRAM_SO = "target/deploy/vault_program.so";
const DECIMALS = 6;

// LiteSVM replaces the RPC layer, so this Connection is never dialled.
export const program = new anchor.Program<VaultProgram>(idl as VaultProgram, {
  connection: new Connection("http://127.0.0.1:8899"),
} as anchor.Provider);

/** Mirrors `seeds = [VAULT_SEED, authority]` in the program. */
export const vaultPda = (authority: PublicKey): [PublicKey, number] =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), authority.toBuffer()],
    PROGRAM_ID
  );

export class Env {
  svm: LiteSVM;
  mint: PublicKey;
  private payer: Keypair;

  constructor() {
    // History off so identical transactions aren't rejected as duplicates.
    this.svm = new LiteSVM().withTransactionHistory(0n);
    this.svm.addProgramFromFile(PROGRAM_ID, PROGRAM_SO);
    this.payer = this.fundedKeypair();

    const mintKp = Keypair.generate();
    this.expectOk(
      this.send(
        [
          SystemProgram.createAccount({
            fromPubkey: this.payer.publicKey,
            newAccountPubkey: mintKp.publicKey,
            lamports: Number(
              this.svm.minimumBalanceForRentExemption(BigInt(MINT_SIZE))
            ),
            space: MINT_SIZE,
            programId: TOKEN_PROGRAM_ID,
          }),
          createInitializeMint2Instruction(
            mintKp.publicKey,
            DECIMALS,
            this.payer.publicKey,
            null
          ),
        ],
        [this.payer, mintKp]
      )
    );
    this.mint = mintKp.publicKey;
  }

  /** Signs and submits a transaction; `signers[0]` pays the fee. */
  send(ixs: TransactionInstruction[], signers: Keypair[]) {
    const tx = new Transaction();
    tx.recentBlockhash = this.svm.latestBlockhash();
    tx.feePayer = signers[0].publicKey;
    tx.add(...ixs);
    tx.sign(...signers);
    return this.svm.sendTransaction(tx);
  }

  /** Fails with program logs instead of a bare boolean. */
  expectOk<T>(res: T | FailedTransactionMetadata): T {
    if (res instanceof FailedTransactionMetadata) {
      throw new Error(
        `transaction failed: ${res.err()}\n${res.meta().logs().join("\n")}`
      );
    }
    return res;
  }

  fundedKeypair(sol = 100): Keypair {
    const kp = Keypair.generate();
    this.svm.airdrop(kp.publicKey, BigInt(sol * LAMPORTS_PER_SOL));
    return kp;
  }

  ata(owner: PublicKey): PublicKey {
    return getAssociatedTokenAddressSync(this.mint, owner, true);
  }

  /** Creates `owner`'s token account and mints `amount` into it. */
  fundTokens(owner: PublicKey, amount: number | bigint): PublicKey {
    const ata = this.ata(owner);
    this.expectOk(
      this.send(
        [
          createAssociatedTokenAccountInstruction(
            this.payer.publicKey,
            ata,
            owner,
            this.mint
          ),
          createMintToInstruction(this.mint, ata, this.payer.publicKey, amount),
        ],
        [this.payer]
      )
    );
    return ata;
  }

  tokenBalance(ata: PublicKey): bigint {
    const account = this.svm.getAccount(ata);
    if (!account) throw new Error(`token account ${ata} missing`);
    return AccountLayout.decode(Buffer.from(account.data)).amount;
  }

  vaultState(vault: PublicKey) {
    const account = this.svm.getAccount(vault);
    if (!account) throw new Error(`vault ${vault} missing`);
    return program.coder.accounts.decode("vault", Buffer.from(account.data));
  }

  async initialize(authority: Keypair) {
    const [vault] = vaultPda(authority.publicKey);
    const ix = await program.methods
      .initialize()
      .accountsPartial({
        authority: authority.publicKey,
        vault,
        systemProgram: SystemProgram.programId,
        mint: this.mint,
        vaultTokenAccount: this.ata(vault),
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .instruction();
    return this.send([ix], [authority]);
  }

  async deposit(authority: Keypair, amount: number | bigint) {
    const [vault] = vaultPda(authority.publicKey);
    const ix = await program.methods
      .deposit(new anchor.BN(amount.toString()))
      .accountsPartial({
        authority: authority.publicKey,
        userTokenAccount: this.ata(authority.publicKey),
        vault,
        vaultTokenAccount: this.ata(vault),
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    return this.send([ix], [authority]);
  }

  async withdraw(authority: Keypair, amount: number | bigint) {
    return this.withdrawFrom(authority, vaultPda(authority.publicKey)[0], amount);
  }

  /** Withdraw naming an explicit vault, so tests can target someone else's. */
  async withdrawFrom(
    authority: Keypair,
    vault: PublicKey,
    amount: number | bigint
  ) {
    const ix = await program.methods
      .withdraw(new anchor.BN(amount.toString()))
      .accountsPartial({
        authority: authority.publicKey,
        vault,
        vaultTokenAccount: this.ata(vault),
        userTokenAccount: this.ata(authority.publicKey),
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    return this.send([ix], [authority]);
  }

  async close(authority: Keypair) {
    return this.closeVault(authority, vaultPda(authority.publicKey)[0]);
  }

  async closeVault(authority: Keypair, vault: PublicKey) {
    const ix = await program.methods
      .close()
      .accountsPartial({
        authority: authority.publicKey,
        vault,
        vaultTokenAccount: this.ata(vault),
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    return this.send([ix], [authority]);
  }
}
