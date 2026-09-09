#![allow(dead_code)] // each test binary uses a subset of these helpers

use anchor_lang::{
    prelude::Pubkey,
    solana_program::{
        instruction::Instruction, program_pack::Pack, system_instruction, system_program,
    },
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::{
        get_associated_token_address, spl_associated_token_account, ID as ASSOCIATED_TOKEN_ID,
    },
    token::spl_token,
};
use litesvm::{types::TransactionResult, LiteSVM};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use vault_program::{state::Vault, VAULT_SEED};

pub const SOL: u64 = 1_000_000_000;
pub const DECIMALS: u8 = 6;

/// Mirrors `seeds = [VAULT_SEED, authority]` in the program.
pub fn vault_pda(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED, authority.as_ref()], &vault_program::ID)
}

pub struct Env {
    pub svm: LiteSVM,
    pub mint: Pubkey,
    payer: Keypair,
}

impl Env {
    pub fn new() -> Self {
        // History off so identical transactions aren't rejected as duplicates.
        let mut svm = LiteSVM::new().with_transaction_history(0);
        svm.add_program_from_file(vault_program::ID, "../../target/deploy/vault_program.so")
            .expect("run `anchor build` first");

        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 100 * SOL).unwrap();

        let mint_kp = Keypair::new();
        let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN);
        send(
            &mut svm,
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &mint_kp.pubkey(),
                    rent,
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::ID,
                ),
                spl_token::instruction::initialize_mint2(
                    &spl_token::ID,
                    &mint_kp.pubkey(),
                    &payer.pubkey(),
                    None,
                    DECIMALS,
                )
                .unwrap(),
            ],
            &[&payer, &mint_kp],
        )
        .unwrap();

        Self {
            svm,
            mint: mint_kp.pubkey(),
            payer,
        }
    }

    pub fn funded_keypair(&mut self) -> Keypair {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 100 * SOL).unwrap();
        kp
    }

    pub fn ata(&self, owner: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, &self.mint)
    }

    /// Creates `owner`'s token account and mints `amount` into it.
    pub fn fund_tokens(&mut self, owner: &Pubkey, amount: u64) -> Pubkey {
        let ata = self.ata(owner);
        let create = spl_associated_token_account::instruction::create_associated_token_account(
            &self.payer.pubkey(),
            owner,
            &self.mint,
            &spl_token::ID,
        );
        let mint_to = spl_token::instruction::mint_to(
            &spl_token::ID,
            &self.mint,
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
        let account = self.svm.get_account(ata).expect("token account missing");
        spl_token::state::Account::unpack(&account.data)
            .unwrap()
            .amount
    }

    pub fn vault_state(&self, vault: &Pubkey) -> Vault {
        let account = self.svm.get_account(vault).expect("vault missing");
        Vault::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn initialize(&mut self, authority: &Keypair) -> TransactionResult {
        let (vault, _) = vault_pda(&authority.pubkey());
        let ix = Instruction {
            program_id: vault_program::ID,
            accounts: vault_program::accounts::Initialize {
                authority: authority.pubkey(),
                vault,
                system_program: system_program::ID,
                mint: self.mint,
                vault_token_account: get_associated_token_address(&vault, &self.mint),
                token_program: spl_token::ID,
                associated_token_program: ASSOCIATED_TOKEN_ID,
            }
            .to_account_metas(None),
            data: vault_program::instruction::Initialize {}.data(),
        };
        send(&mut self.svm, &[ix], &[authority])
    }

    pub fn deposit(&mut self, authority: &Keypair, amount: u64) -> TransactionResult {
        let (vault, _) = vault_pda(&authority.pubkey());
        let ix = Instruction {
            program_id: vault_program::ID,
            accounts: vault_program::accounts::Deposit {
                authority: authority.pubkey(),
                user_token_account: self.ata(&authority.pubkey()),
                vault,
                vault_token_account: get_associated_token_address(&vault, &self.mint),
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: vault_program::instruction::Deposit { amount }.data(),
        };
        send(&mut self.svm, &[ix], &[authority])
    }

    pub fn withdraw(&mut self, authority: &Keypair, amount: u64) -> TransactionResult {
        self.withdraw_from(authority, &vault_pda(&authority.pubkey()).0, amount)
    }

    /// Withdraw naming an explicit vault, so tests can target someone else's.
    pub fn withdraw_from(
        &mut self,
        authority: &Keypair,
        vault: &Pubkey,
        amount: u64,
    ) -> TransactionResult {
        let ix = Instruction {
            program_id: vault_program::ID,
            accounts: vault_program::accounts::Withdraw {
                authority: authority.pubkey(),
                vault: *vault,
                vault_token_account: get_associated_token_address(vault, &self.mint),
                user_token_account: self.ata(&authority.pubkey()),
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: vault_program::instruction::Withdraw { amount }.data(),
        };
        send(&mut self.svm, &[ix], &[authority])
    }

    pub fn close(&mut self, authority: &Keypair) -> TransactionResult {
        self.close_vault(authority, &vault_pda(&authority.pubkey()).0)
    }

    pub fn close_vault(&mut self, authority: &Keypair, vault: &Pubkey) -> TransactionResult {
        let ix = Instruction {
            program_id: vault_program::ID,
            accounts: vault_program::accounts::Close {
                authority: authority.pubkey(),
                vault: *vault,
                vault_token_account: get_associated_token_address(vault, &self.mint),
                user_token_account: self.ata(&authority.pubkey()),
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: vault_program::instruction::Close {}.data(),
        };
        send(&mut self.svm, &[ix], &[authority])
    }
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
