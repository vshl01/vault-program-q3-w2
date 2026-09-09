#![allow(dead_code)] // each test binary uses a subset of these helpers

use anchor_lang::{
    prelude::Pubkey,
    solana_program::{
        clock::Clock, instruction::Instruction, program_pack::Pack, system_instruction,
        system_program,
    },
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::{
        get_associated_token_address, spl_associated_token_account, ID as ASSOCIATED_TOKEN_ID,
    },
    token::spl_token,
};
use escrow::{state::Escrow, ESCROW_SEED};
use litesvm::{types::TransactionResult, LiteSVM};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const SOL: u64 = 1_000_000_000;
pub const HOUR: i64 = 3_600;

/// Mirrors `seeds = [ESCROW_SEED, maker, seed]` in the program.
pub fn escrow_pda(maker: &Pubkey, seed: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ESCROW_SEED, maker.as_ref(), &seed.to_le_bytes()],
        &escrow::ID,
    )
}

pub struct Env {
    pub svm: LiteSVM,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    payer: Keypair,
}

impl Env {
    pub fn new() -> Self {
        // History off so identical transactions aren't rejected as duplicates.
        let mut svm = LiteSVM::new().with_transaction_history(0);
        svm.add_program_from_file(escrow::ID, "../../target/deploy/escrow.so")
            .expect("run `anchor build` first");

        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 1_000 * SOL).unwrap();

        let mint_a = create_mint(&mut svm, &payer);
        let mint_b = create_mint(&mut svm, &payer);

        Self {
            svm,
            mint_a,
            mint_b,
            payer,
        }
    }

    pub fn now(&self) -> i64 {
        self.svm.get_sysvar::<Clock>().unix_timestamp
    }

    /// Moves the clock forward so deadline behaviour can be exercised.
    pub fn warp(&mut self, seconds: i64) {
        let mut clock = self.svm.get_sysvar::<Clock>();
        clock.unix_timestamp += seconds;
        self.svm.set_sysvar(&clock);
    }

    pub fn funded_keypair(&mut self) -> Keypair {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 100 * SOL).unwrap();
        kp
    }

    pub fn ata(&self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, mint)
    }

    /// Creates `owner`'s token account for `mint` and mints `amount` into it.
    pub fn fund_tokens(&mut self, owner: &Pubkey, mint: &Pubkey, amount: u64) -> Pubkey {
        let ata = self.ata(owner, mint);
        let create = spl_associated_token_account::instruction::create_associated_token_account(
            &self.payer.pubkey(),
            owner,
            mint,
            &spl_token::ID,
        );
        let mint_to = spl_token::instruction::mint_to(
            &spl_token::ID,
            mint,
            &ata,
            &self.payer.pubkey(),
            &[],
            amount,
        )
        .unwrap();
        send(&mut self.svm, &[create, mint_to], &[&self.payer]).unwrap();
        ata
    }

    pub fn token_balance(&self, ata: &Pubkey) -> u64 {
        self.svm
            .get_account(ata)
            .map(|a| spl_token::state::Account::unpack(&a.data).unwrap().amount)
            .unwrap_or(0)
    }

    pub fn escrow_state(&self, escrow: &Pubkey) -> Escrow {
        let account = self.svm.get_account(escrow).expect("escrow missing");
        Escrow::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn is_closed(&self, address: &Pubkey) -> bool {
        self.svm
            .get_account(address)
            .is_none_or(|a| a.data.is_empty())
    }

    pub fn make(
        &mut self,
        maker: &Keypair,
        seed: u64,
        deposit: u64,
        receive: u64,
        deadline: i64,
    ) -> TransactionResult {
        let (escrow, _) = escrow_pda(&maker.pubkey(), seed);
        let ix = Instruction {
            program_id: escrow::ID,
            accounts: escrow::accounts::Make {
                maker: maker.pubkey(),
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                maker_ata_a: self.ata(&maker.pubkey(), &self.mint_a),
                escrow,
                vault: self.ata(&escrow, &self.mint_a),
                token_program: spl_token::ID,
                associated_token_program: ASSOCIATED_TOKEN_ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: escrow::instruction::Make {
                seed,
                deposit,
                receive,
                deadline,
            }
            .data(),
        };
        send(&mut self.svm, &[ix], &[maker])
    }

    pub fn take(
        &mut self,
        taker: &Keypair,
        maker: &Pubkey,
        seed: u64,
        expected_receive: u64,
    ) -> TransactionResult {
        let (escrow, _) = escrow_pda(maker, seed);
        let ix = Instruction {
            program_id: escrow::ID,
            accounts: escrow::accounts::Take {
                taker: taker.pubkey(),
                maker: *maker,
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                taker_ata_a: self.ata(&taker.pubkey(), &self.mint_a),
                taker_ata_b: self.ata(&taker.pubkey(), &self.mint_b),
                maker_ata_b: self.ata(maker, &self.mint_b),
                escrow,
                vault: self.ata(&escrow, &self.mint_a),
                token_program: spl_token::ID,
                associated_token_program: ASSOCIATED_TOKEN_ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: escrow::instruction::Take { expected_receive }.data(),
        };
        send(&mut self.svm, &[ix], &[taker])
    }

    pub fn refund(&mut self, maker: &Keypair, seed: u64) -> TransactionResult {
        self.refund_escrow(maker, &escrow_pda(&maker.pubkey(), seed).0)
    }

    /// Refund naming an explicit escrow, so tests can target someone else's.
    pub fn refund_escrow(&mut self, maker: &Keypair, escrow: &Pubkey) -> TransactionResult {
        let escrow = *escrow;
        let ix = Instruction {
            program_id: escrow::ID,
            accounts: escrow::accounts::Refund {
                maker: maker.pubkey(),
                mint_a: self.mint_a,
                maker_ata_a: self.ata(&maker.pubkey(), &self.mint_a),
                escrow,
                vault: self.ata(&escrow, &self.mint_a),
                token_program: spl_token::ID,
                associated_token_program: ASSOCIATED_TOKEN_ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: escrow::instruction::Refund {}.data(),
        };
        send(&mut self.svm, &[ix], &[maker])
    }

    pub fn update(&mut self, maker: &Keypair, seed: u64, receive: u64) -> TransactionResult {
        self.update_escrow(maker, &escrow_pda(&maker.pubkey(), seed).0, receive)
    }

    /// Update naming an explicit escrow, so tests can target someone else's.
    pub fn update_escrow(
        &mut self,
        maker: &Keypair,
        escrow: &Pubkey,
        receive: u64,
    ) -> TransactionResult {
        let escrow = *escrow;
        let ix = Instruction {
            program_id: escrow::ID,
            accounts: escrow::accounts::Update {
                maker: maker.pubkey(),
                escrow,
            }
            .to_account_metas(None),
            data: escrow::instruction::Update { receive }.data(),
        };
        send(&mut self.svm, &[ix], &[maker])
    }
}

fn create_mint(svm: &mut LiteSVM, payer: &Keypair) -> Pubkey {
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN);
    send(
        svm,
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::ID,
            ),
            spl_token::instruction::initialize_mint2(
                &spl_token::ID,
                &mint.pubkey(),
                &payer.pubkey(),
                None,
                6,
            )
            .unwrap(),
        ],
        &[payer, &mint],
    )
    .unwrap();
    mint.pubkey()
}

/// Signs and submits a transaction; `signers[0]` pays the fee.
pub fn send(svm: &mut LiteSVM, ixs: &[Instruction], signers: &[&Keypair]) -> TransactionResult {
    let payer = signers[0].pubkey();
    let tx = Transaction::new(
        signers,
        Message::new(ixs, Some(&payer)),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
}
