use anchor_lang::{
    prelude::Pubkey,
    solana_program::{
        instruction::Instruction, program_pack::Pack, system_instruction, system_program,
    },
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{associated_token, token::spl_token};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use vault_program::{state::Vault, VAULT_SEED};

const PROGRAM_SO: &str = "../../target/deploy/vault_program.so";

/// Mirrors the `seeds = [VAULT_SEED, authority]` constraint in the program.
fn vault_pda(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED, authority.as_ref()], &vault_program::ID)
}

/// Boots a LiteSVM instance with the vault program loaded, a funded authority
/// and a freshly created SPL mint.
fn setup() -> (LiteSVM, Keypair, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(vault_program::ID, PROGRAM_SO)
        .expect("build the program first: `anchor build`");

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 100 * 1_000_000_000)
        .unwrap();

    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN);
    let create_mint = system_instruction::create_account(
        &authority.pubkey(),
        &mint.pubkey(),
        rent,
        spl_token::state::Mint::LEN as u64,
        &spl_token::ID,
    );
    let init_mint = spl_token::instruction::initialize_mint2(
        &spl_token::ID,
        &mint.pubkey(),
        &authority.pubkey(),
        None,
        6,
    )
    .unwrap();

    let tx = Transaction::new(
        &[&authority, &mint],
        Message::new(&[create_mint, init_mint], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    (svm, authority, mint)
}

/// Builds the `initialize` instruction for the given authority/mint.
fn initialize_ix(authority: &Keypair, mint: &Keypair) -> Instruction {
    let (vault, _) = vault_pda(&authority.pubkey());
    let vault_token_account =
        associated_token::get_associated_token_address(&vault, &mint.pubkey());

    Instruction {
        program_id: vault_program::ID,
        accounts: vault_program::accounts::Initialize {
            authority: authority.pubkey(),
            vault,
            system_program: system_program::ID,
            mint: mint.pubkey(),
            vault_token_account,
            token_program: spl_token::ID,
            associated_token_program: associated_token::ID,
        }
        .to_account_metas(None),
        data: vault_program::instruction::Initialize {}.data(),
    }
}

#[test]
fn initialize_creates_vault_and_token_account() {
    let (mut svm, authority, mint) = setup();

    let (vault_pda, expected_bump) = vault_pda(&authority.pubkey());
    let vault_ata = associated_token::get_associated_token_address(&vault_pda, &mint.pubkey());

    let tx = Transaction::new(
        &[&authority],
        Message::new(&[initialize_ix(&authority, &mint)], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // The vault PDA is owned by our program and holds the expected state.
    let vault_account = svm.get_account(&vault_pda).expect("vault was not created");
    assert_eq!(vault_account.owner, vault_program::ID);

    let vault = Vault::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.authority, authority.pubkey());
    assert_eq!(vault.bump, expected_bump);

    // The vault ATA exists, is owned by the token program, and is empty.
    let ata_account = svm.get_account(&vault_ata).expect("vault ATA was not created");
    assert_eq!(ata_account.owner, spl_token::ID);

    let token_account = spl_token::state::Account::unpack(&ata_account.data).unwrap();
    assert_eq!(token_account.mint, mint.pubkey());
    assert_eq!(token_account.owner, vault_pda, "ATA authority must be the vault PDA");
    assert_eq!(token_account.amount, 0);
}

#[test]
fn initialize_twice_fails() {
    let (mut svm, authority, mint) = setup();

    let tx = Transaction::new(
        &[&authority],
        Message::new(&[initialize_ix(&authority, &mint)], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Same authority => same vault PDA => `init` must reject the second call.
    svm.expire_blockhash();
    let tx = Transaction::new(
        &[&authority],
        Message::new(&[initialize_ix(&authority, &mint)], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );
    assert!(
        svm.send_transaction(tx).is_err(),
        "re-initializing an existing vault should fail"
    );
}

#[test]
fn initialize_is_per_authority() {
    let (mut svm, authority, mint) = setup();

    let tx = Transaction::new(
        &[&authority],
        Message::new(&[initialize_ix(&authority, &mint)], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // A different authority derives a different vault and can initialize freely.
    let other = Keypair::new();
    svm.airdrop(&other.pubkey(), 100 * 1_000_000_000).unwrap();

    let tx = Transaction::new(
        &[&other],
        Message::new(&[initialize_ix(&other, &mint)], Some(&other.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (other_vault, _) = vault_pda(&other.pubkey());
    let vault = Vault::try_deserialize(
        &mut svm.get_account(&other_vault).unwrap().data.as_slice(),
    )
    .unwrap();
    assert_eq!(vault.authority, other.pubkey());
}
