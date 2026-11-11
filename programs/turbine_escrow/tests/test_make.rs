mod common;

use {
    anchor_lang::AccountDeserialize,
    common::*,
    solana_signer::Signer,
};

#[test]
fn test_make_success() {
    let MakeFixture {
        mut svm,
        maker,
        mint_a,
        mint_b,
        maker_ata_a,
        escrow,
        escrow_bump,
        vault,
    } = setup();
    let maker_pubkey = maker.pubkey();

    let ix = make_tx(
        &maker_pubkey,
        &mint_a,
        &mint_b,
        &maker_ata_a,
        &escrow,
        &vault,
        SEED,
        DEPOSIT,
        RECEIVE,
    );
    send(&mut svm, &[&maker], ix).expect("make should succeed");

    // Escrow state was initialized correctly.
    let acct = svm.get_account(&escrow).unwrap();
    let mut data: &[u8] = &acct.data;
    let state = turbine_escrow::state::Escrow::try_deserialize(&mut data).unwrap();
    assert_eq!(state.seed, SEED);
    assert_eq!(state.maker, maker_pubkey);
    assert_eq!(state.mint_a, mint_a);
    assert_eq!(state.mint_b, mint_b);
    assert_eq!(state.receive, RECEIVE);
    assert_eq!(state.bump, escrow_bump);

    // Vault received the deposit, maker ATA was debited.
    assert_eq!(token_amount(&svm, &vault), DEPOSIT);
    assert_eq!(
        token_amount(&svm, &maker_ata_a),
        MAKER_ATA_AMOUNT - DEPOSIT
    );
}

#[test]
fn test_make_zero_deposit_fails() {
    let MakeFixture {
        mut svm,
        maker,
        mint_a,
        mint_b,
        maker_ata_a,
        escrow,
        vault,
        ..
    } = setup();
    let maker_pubkey = maker.pubkey();

    let ix = make_tx(
        &maker_pubkey,
        &mint_a,
        &mint_b,
        &maker_ata_a,
        &escrow,
        &vault,
        SEED,
        0,
        RECEIVE,
    );
    let err = send(&mut svm, &[&maker], ix).unwrap_err();

    let expected = turbine_escrow::error::EscrowError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected InvalidAmount, got: {err_str}"
    );
}

#[test]
fn test_make_zero_receive_fails() {
    let MakeFixture {
        mut svm,
        maker,
        mint_a,
        mint_b,
        maker_ata_a,
        escrow,
        vault,
        ..
    } = setup();
    let maker_pubkey = maker.pubkey();

    let ix = make_tx(
        &maker_pubkey,
        &mint_a,
        &mint_b,
        &maker_ata_a,
        &escrow,
        &vault,
        SEED,
        DEPOSIT,
        0,
    );
    let err = send(&mut svm, &[&maker], ix).unwrap_err();

    let expected = turbine_escrow::error::EscrowError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected InvalidAmount, got: {err_str}"
    );
}
