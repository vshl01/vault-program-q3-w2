#![allow(dead_code)] // each test binary uses a subset of these helpers

use anchor_lang::{
    prelude::Pubkey,
    solana_program::{instruction::Instruction, program_pack::Pack, system_instruction, system_program},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::{
        get_associated_token_address, spl_associated_token_account, ID as ASSOCIATED_TOKEN_ID,
    },
    token::spl_token,
};
use litesvm::{types::TransactionResult, LiteSVM};
use nc_vault::{state::Vault, SHARE_SEED, VAULT_SEED};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const SOL: u64 = 1_000_000_000;

pub struct Env {
    pub svm: LiteSVM,
    pub base_mint: Pubkey,
    pub position_mint: Pubkey,
    pub manager: Keypair,
    payer: Keypair,
}

impl Env {
    /// Boots a vault that is already initialized and empty.
    pub fn new() -> Self {
        let mut svm = LiteSVM::new().with_transaction_history(0);
        svm.add_program_from_file(nc_vault::ID, "../../target/deploy/nc_vault.so")
            .expect("run `anchor build` first");

        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 1_000 * SOL).unwrap();
        let base_mint = create_mint(&mut svm, &payer);
        let position_mint = create_mint(&mut svm, &payer);

        let manager = Keypair::new();
        svm.airdrop(&manager.pubkey(), 100 * SOL).unwrap();

        let mut env = Self {
            svm,
            base_mint,
            position_mint,
            manager,
            payer,
        };
        env.initialize().unwrap();
        env
    }

    pub fn vault(&self) -> Pubkey {
        Pubkey::find_program_address(
            &[
                VAULT_SEED,
                self.base_mint.as_ref(),
                self.position_mint.as_ref(),
            ],
            &nc_vault::ID,
        )
        .0
    }

    pub fn share_mint(&self) -> Pubkey {
        Pubkey::find_program_address(&[SHARE_SEED, self.vault().as_ref()], &nc_vault::ID).0
    }

    pub fn base_vault(&self) -> Pubkey {
        get_associated_token_address(&self.vault(), &self.base_mint)
    }

    pub fn position_vault(&self) -> Pubkey {
        get_associated_token_address(&self.vault(), &self.position_mint)
    }

    pub fn vault_state(&self) -> Vault {
        let account = self.svm.get_account(&self.vault()).expect("vault missing");
        Vault::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn funded_keypair(&mut self) -> Keypair {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 100 * SOL).unwrap();
        kp
    }

    pub fn ata(&self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        get_associated_token_address(owner, mint)
    }

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

    pub fn balance(&self, ata: &Pubkey) -> u64 {
        self.svm
            .get_account(ata)
            .map(|a| spl_token::state::Account::unpack(&a.data).unwrap().amount)
            .unwrap_or(0)
    }

    pub fn shares_of(&self, owner: &Pubkey) -> u64 {
        self.balance(&self.ata(owner, &self.share_mint()))
    }

    pub fn share_supply(&self) -> u64 {
        self.svm
            .get_account(&self.share_mint())
            .map(|a| spl_token::state::Mint::unpack(&a.data).unwrap().supply)
            .unwrap_or(0)
    }

    fn initialize(&mut self) -> TransactionResult {
        let ix = Instruction {
            program_id: nc_vault::ID,
            accounts: nc_vault::accounts::Initialize {
                manager: self.manager.pubkey(),
                base_mint: self.base_mint,
                position_mint: self.position_mint,
                vault: self.vault(),
                share_mint: self.share_mint(),
                base_vault: self.base_vault(),
                position_vault: self.position_vault(),
                token_program: spl_token::ID,
                associated_token_program: ASSOCIATED_TOKEN_ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: nc_vault::instruction::Initialize {}.data(),
        };
        send(&mut self.svm, &[ix], &[&self.manager])
    }

    /// A user holding `amount` of base, with a share account ready.
    pub fn depositor(&mut self, amount: u64) -> Keypair {
        let user = self.funded_keypair();
        self.fund_tokens(&user.pubkey(), &self.base_mint.clone(), amount);
        user
    }

    pub fn deposit(&mut self, user: &Keypair, amount: u64) -> TransactionResult {
        let ix = Instruction {
            program_id: nc_vault::ID,
            accounts: nc_vault::accounts::Deposit {
                user: user.pubkey(),
                vault: self.vault(),
                base_mint: self.base_mint,
                position_mint: self.position_mint,
                share_mint: self.share_mint(),
                base_vault: self.base_vault(),
                position_vault: self.position_vault(),
                user_base: self.ata(&user.pubkey(), &self.base_mint),
                user_shares: self.ata(&user.pubkey(), &self.share_mint()),
                token_program: spl_token::ID,
                associated_token_program: ASSOCIATED_TOKEN_ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: nc_vault::instruction::Deposit { amount }.data(),
        };
        send(&mut self.svm, &[ix], &[user])
    }

    /// A counterparty holding `position` tokens and an empty base account.
    pub fn counterparty(&mut self, position: u64) -> Keypair {
        let cp = self.funded_keypair();
        self.fund_tokens(&cp.pubkey(), &self.position_mint.clone(), position);
        self.fund_tokens(&cp.pubkey(), &self.base_mint.clone(), 0);
        cp
    }

    pub fn deploy(
        &mut self,
        manager: &Keypair,
        counterparty: &Keypair,
        base_out: u64,
        position_in: u64,
    ) -> TransactionResult {
        let ix = Instruction {
            program_id: nc_vault::ID,
            accounts: nc_vault::accounts::Deploy {
                manager: manager.pubkey(),
                counterparty: counterparty.pubkey(),
                vault: self.vault(),
                base_mint: self.base_mint,
                position_mint: self.position_mint,
                base_vault: self.base_vault(),
                position_vault: self.position_vault(),
                counterparty_base: self.ata(&counterparty.pubkey(), &self.base_mint),
                counterparty_position: self.ata(&counterparty.pubkey(), &self.position_mint),
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: nc_vault::instruction::Deploy {
                base_out,
                position_in,
            }
            .data(),
        };
        send(&mut self.svm, &[ix], &[manager, counterparty])
    }

    pub fn redeem(&mut self, user: &Keypair, shares: u64) -> TransactionResult {
        let ix = Instruction {
            program_id: nc_vault::ID,
            accounts: nc_vault::accounts::Redeem {
                user: user.pubkey(),
                vault: self.vault(),
                base_mint: self.base_mint,
                position_mint: self.position_mint,
                share_mint: self.share_mint(),
                base_vault: self.base_vault(),
                position_vault: self.position_vault(),
                user_base: self.ata(&user.pubkey(), &self.base_mint),
                user_position: self.ata(&user.pubkey(), &self.position_mint),
                user_shares: self.ata(&user.pubkey(), &self.share_mint()),
                token_program: spl_token::ID,
                associated_token_program: ASSOCIATED_TOKEN_ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: nc_vault::instruction::Redeem { shares }.data(),
        };
        send(&mut self.svm, &[ix], &[user])
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

pub fn send(svm: &mut LiteSVM, ixs: &[Instruction], signers: &[&Keypair]) -> TransactionResult {
    let payer = signers[0].pubkey();
    let tx = Transaction::new(
        signers,
        Message::new(ixs, Some(&payer)),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
}
