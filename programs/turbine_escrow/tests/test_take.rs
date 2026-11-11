mod common;

use {
    anchor_spl::associated_token::get_associated_token_address_with_program_id, common::*,
    solana_signer::Signer, spl_token_interface::ID as TOKEN_PROGRAM_ID,
    turbine_escrow::error::EscrowError,
};

#[test]
fn test_take_success() {
    let MakeFixture { mut svm, accounts } = setup();

    // Establish the escrow
    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    // Take is permissionless by design: anyone funded can accept
    let take = setup_take(&mut svm, &accounts);
    let ix = take_tx(&accounts, &take);
    send(&mut svm, &[&take.taker], ix).expect("take should succeed");

    // Taker got the deposit of A, maker got the receive amount of B
    assert_eq!(token_amount(&svm, &take.taker_ata_a), DEPOSIT);
    assert_eq!(token_amount(&svm, &take.maker_ata_b), RECEIVE);
    assert_eq!(
        token_amount(&svm, &take.taker_ata_b),
        TAKER_B_FUNDING - RECEIVE
    );

    // Both escrow and vault closed
    assert!(svm.get_account(&accounts.escrow).is_none());
    assert!(svm.get_account(&accounts.vault).is_none());
}

#[test]
fn test_take_insufficient_balance_fails() {
    let MakeFixture { mut svm, accounts } = setup();

    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    // Fund the taker with one base unit less than the receive amount
    let take = setup_take_with_funding(&mut svm, &accounts, RECEIVE - 1);
    let ix = take_tx(&accounts, &take);
    let err = send(&mut svm, &[&take.taker], ix).unwrap_err();

    let expected = EscrowError::InsufficientBalance.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected InsufficientBalance, got: {err_str}"
    );
}

#[test]
fn test_take_wrong_mint_fails() {
    let MakeFixture { mut svm, accounts } = setup();

    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    let take = setup_take(&mut svm, &accounts);

    // Foreign mint with all B-side addresses re-derived for it
    // Needed to trigger escrow's has_one Mint B check
    let mint_c = create_mint(&mut svm, &accounts.maker);
    let taker_pubkey = take.taker.pubkey();
    let taker_ata_c = fund_ata(
        &mut svm,
        &take.taker,
        &accounts.maker,
        &mint_c,
        &taker_pubkey,
        RECEIVE,
    );
    let maker_ata_c = get_associated_token_address_with_program_id(
        &accounts.maker.pubkey(),
        &mint_c,
        &TOKEN_PROGRAM_ID,
    );
    let taker_ata_b = take.taker_ata_b;
    let take_c = TakeSetup {
        taker: take.taker,
        taker_ata_a: take.taker_ata_a,
        taker_ata_b: taker_ata_c,
        maker_ata_b: maker_ata_c,
    };
    let ix = take_tx_with_mints(&accounts, &take_c, &accounts.mint_a, &mint_c);
    let err = send(&mut svm, &[&take_c.taker], ix).unwrap_err();

    let expected = EscrowError::InvalidMintB.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected InvalidMintB, got: {err_str}"
    );

    // Nothing moved: the failed take left all balances untouched
    assert_eq!(token_amount(&svm, &accounts.vault), DEPOSIT);
    assert_eq!(token_amount(&svm, &taker_ata_b), TAKER_B_FUNDING);
}

#[test]
fn test_take_twice_fails() {
    let MakeFixture { mut svm, accounts } = setup();

    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    let take = setup_take(&mut svm, &accounts);
    let ix = take_tx(&accounts, &take);
    send(&mut svm, &[&take.taker], ix).expect("take should succeed");

    // Duplicate take should fail
    let ix = take_tx(&accounts, &take);
    let err = send(&mut svm, &[&take.taker], ix).unwrap_err();

    let expected = anchor_lang::error::ErrorCode::AccountNotInitialized.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected AccountNotInitialized, got: {err_str}"
    );
}
