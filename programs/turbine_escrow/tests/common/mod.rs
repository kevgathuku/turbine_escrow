#![allow(dead_code)]

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        InstructionData, ToAccountMetas,
    },
    anchor_spl::associated_token::{
        get_associated_token_address, get_associated_token_address_with_program_id,
        ID as ATA_PROGRAM_ID,
    },
    anchor_spl::token::spl_token::state::Account as TokenAccount,
    litesvm::LiteSVM,
    litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program_pack::Pack,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    turbine_escrow::{accounts, constants::ESCROW_SEED},
};

pub use litesvm_token::spl_token::ID as TOKEN_PROGRAM_ID;

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

pub const SEED: u64 = 42;
// Decimals for both mint tokens
pub const DECIMALS: u8 = 6;
// One token in base units (6 decimals), so amounts read as whole tokens.
pub const TOKEN_UNIT: u64 = 1_000_000;
pub const DEPOSIT: u64 = 100 * TOKEN_UNIT;
pub const RECEIVE: u64 = 200 * TOKEN_UNIT;
pub const MAKER_ATA_AMOUNT: u64 = 1_000 * TOKEN_UNIT;
// Taker's starting balance of token B; must cover RECEIVE.
pub const TAKER_B_FUNDING: u64 = 1_000 * TOKEN_UNIT;

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

// New mint with the maker as authority.
pub fn create_mint(svm: &mut LiteSVM, maker: &Keypair) -> Pubkey {
    let maker_pubkey = maker.pubkey();
    CreateMint::new(svm, maker)
        .decimals(DECIMALS)
        .authority(&maker_pubkey)
        .send()
        .unwrap()
}

// New ATA for `owner` plus `amount` freshly minted into it.
pub fn fund_ata(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = CreateAssociatedTokenAccount::new(svm, payer, mint)
        .owner(owner)
        .send()
        .unwrap();
    MintTo::new(svm, payer, mint, &ata, amount)
        .owner(mint_authority)
        .send()
        .unwrap();
    ata
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

    let mint_a = create_mint(&mut svm, &maker);
    let mint_b = create_mint(&mut svm, &maker);

    // Fund the maker's ATA with token A.
    let maker_ata_a = fund_ata(&mut svm, &maker, &maker, &mint_a, &maker_pubkey, funding);

    let (escrow, escrow_bump) = Pubkey::find_program_address(
        &[
            ESCROW_SEED,
            maker_pubkey.as_ref(),
            SEED.to_le_bytes().as_ref(),
        ],
        &program_id,
    );
    let vault = get_associated_token_address(&escrow, &mint_a);

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

pub struct TakeSetup {
    pub taker: Keypair,
    pub taker_ata_a: Pubkey,
    pub taker_ata_b: Pubkey,
    pub maker_ata_b: Pubkey,
}

// Fund the taker with token B. The receiving ATAs are derived only:
// take creates them on demand via init_if_needed.
pub fn setup_take(svm: &mut LiteSVM, accounts: &MakeAccounts) -> TakeSetup {
    setup_take_with_funding(svm, accounts, TAKER_B_FUNDING)
}

pub fn setup_take_with_funding(
    svm: &mut LiteSVM,
    accounts: &MakeAccounts,
    funding: u64,
) -> TakeSetup {
    let taker = Keypair::new();
    let taker_pubkey = taker.pubkey();
    svm.airdrop(&taker_pubkey, 2 * LAMPORTS_PER_SOL).unwrap();

    let taker_ata_b = fund_ata(
        svm,
        &taker,
        &accounts.maker,
        &accounts.mint_b,
        &taker_pubkey,
        funding,
    );

    let taker_ata_a = get_associated_token_address_with_program_id(
        &taker_pubkey,
        &accounts.mint_a,
        &TOKEN_PROGRAM_ID,
    );
    let maker_ata_b = get_associated_token_address_with_program_id(
        &accounts.maker.pubkey(),
        &accounts.mint_b,
        &TOKEN_PROGRAM_ID,
    );

    TakeSetup {
        taker,
        taker_ata_a,
        taker_ata_b,
        maker_ata_b,
    }
}

pub fn take_tx(accounts: &MakeAccounts, take: &TakeSetup) -> Instruction {
    take_tx_with_mints(accounts, take, &accounts.mint_a, &accounts.mint_b)
}

pub fn take_tx_with_mints(
    accounts: &MakeAccounts,
    take: &TakeSetup,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        turbine_escrow::id(),
        &turbine_escrow::instruction::Take {}.data(),
        accounts::Take {
            taker: take.taker.pubkey(),
            maker: accounts.maker.pubkey(),
            escrow: accounts.escrow,
            mint_a: *mint_a,
            mint_b: *mint_b,
            vault: accounts.vault,
            taker_ata_a: take.taker_ata_a,
            taker_ata_b: take.taker_ata_b,
            maker_ata_b: take.maker_ata_b,
            associated_token_program: ATA_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn refund_tx(accounts: &MakeAccounts) -> Instruction {
    refund_tx_with_maker(accounts, &accounts.maker.pubkey())
}

pub fn refund_tx_with_maker(accounts: &MakeAccounts, maker: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        turbine_escrow::id(),
        &turbine_escrow::instruction::Refund {}.data(),
        accounts::Refund {
            maker: *maker,
            escrow: accounts.escrow,
            mint_a: accounts.mint_a,
            maker_ata_a: accounts.maker_ata_a,
            vault: accounts.vault,
            associated_token_program: ATA_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn update_tx(accounts: &MakeAccounts, receive: u64) -> Instruction {
    Instruction::new_with_bytes(
        turbine_escrow::id(),
        &turbine_escrow::instruction::Update { receive }.data(),
        accounts::Update {
            maker: accounts.maker.pubkey(),
            escrow: accounts.escrow,
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
