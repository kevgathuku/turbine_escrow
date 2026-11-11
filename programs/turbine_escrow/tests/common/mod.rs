#![allow(dead_code)]

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        InstructionData, ToAccountMetas,
    },
    anchor_spl::associated_token::{
        get_associated_token_address_with_program_id, ID as ATA_PROGRAM_ID,
    },
    litesvm::LiteSVM,
    litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program_pack::Pack,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    spl_token_interface::{state::Account as TokenAccount, ID as TOKEN_PROGRAM_ID},
    turbine_escrow::{accounts, constants::ESCROW_SEED},
};

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

pub const SEED: u64 = 42;
// Decimals for both mint tokens
pub const DECIMALS: u8 = 6;
// One token in base units (6 decimals), so amounts read as whole tokens.
pub const TOKEN_UNIT: u64 = 1_000_000;
pub const DEPOSIT: u64 = 100 * TOKEN_UNIT;
pub const RECEIVE: u64 = 200 * TOKEN_UNIT;
pub const MAKER_ATA_AMOUNT: u64 = 1_000 * TOKEN_UNIT;

pub struct MakeAccounts {
    pub maker: Keypair,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub maker_ata_a: Pubkey,
    pub escrow: Pubkey,
    pub escrow_bump: u8,
    pub vault: Pubkey,
}

pub struct MakeFixture {
    pub svm: LiteSVM,
    pub accounts: MakeAccounts,
}

pub fn setup() -> MakeFixture {
    setup_with_funding(MAKER_ATA_AMOUNT)
}

pub fn setup_with_funding(funding: u64) -> MakeFixture {
    let program_id = turbine_escrow::id();
    let maker = Keypair::new();
    let maker_pubkey = maker.pubkey();

    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/turbine_escrow.so"
    ));
    svm.add_program(program_id, program_bytes).unwrap();
    svm.airdrop(&maker_pubkey, 10 * LAMPORTS_PER_SOL).unwrap();

    // Real token setup through the bundled Token/ATA programs: two mints,
    // the maker's funded ATA, and the escrow vault ATA (created empty).
    let mint_a = CreateMint::new(&mut svm, &maker)
        .decimals(DECIMALS)
        .authority(&maker_pubkey)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut svm, &maker)
        .decimals(DECIMALS)
        .authority(&maker_pubkey)
        .send()
        .unwrap();

    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .send()
        .unwrap();
    // Fund the maker's ATA with token A
    MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, funding)
        .send()
        .unwrap();

    let (escrow, escrow_bump) = Pubkey::find_program_address(
        &[
            ESCROW_SEED,
            maker_pubkey.as_ref(),
            SEED.to_le_bytes().as_ref(),
        ],
        &program_id,
    );
    let vault = get_associated_token_address_with_program_id(&escrow, &mint_a, &TOKEN_PROGRAM_ID);

    MakeFixture {
        svm,
        accounts: MakeAccounts {
            maker,
            mint_a,
            mint_b,
            maker_ata_a,
            escrow,
            escrow_bump,
            vault,
        },
    }
}

pub fn make_tx(accounts: &MakeAccounts, seed: u64, deposit: u64, receive: u64) -> Instruction {
    make_tx_with_mints(
        accounts,
        &accounts.mint_a,
        &accounts.mint_b,
        seed,
        deposit,
        receive,
    )
}

pub fn make_tx_with_mints(
    accounts: &MakeAccounts,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    seed: u64,
    deposit: u64,
    receive: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        turbine_escrow::id(),
        &turbine_escrow::instruction::Make {
            seed,
            deposit,
            receive,
        }
        .data(),
        accounts::Make {
            maker: accounts.maker.pubkey(),
            mint_a: *mint_a,
            mint_b: *mint_b,
            maker_ata_a: accounts.maker_ata_a,
            escrow: accounts.escrow,
            vault: accounts.vault,
            associated_token_program: ATA_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn token_amount(svm: &LiteSVM, address: &Pubkey) -> u64 {
    let acct = svm.get_account(address).unwrap();
    TokenAccount::unpack(&acct.data).unwrap().amount
}

#[allow(clippy::result_large_err)]
pub fn send(
    svm: &mut LiteSVM,
    signers: &[&Keypair],
    ix: Instruction,
) -> litesvm::types::TransactionResult {
    svm.expire_blockhash();

    let payer = signers[0].pubkey();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();

    svm.send_transaction(tx)
}
