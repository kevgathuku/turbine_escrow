mod common;

use {
    anchor_lang::AccountDeserialize, common::*, solana_signer::Signer,
    turbine_escrow::error::EscrowError,
};

#[test]
fn test_make_success() {
    let MakeFixture { mut svm, accounts } = setup();
    let maker_pubkey = &accounts.maker.pubkey();

    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    // Escrow state was initialized correctly.
    let acct = svm.get_account(&accounts.escrow).unwrap();
    let mut data: &[u8] = &acct.data;
    let state = turbine_escrow::state::Escrow::try_deserialize(&mut data).unwrap();
    assert_eq!(state.seed, SEED);
    assert_eq!(state.maker, *maker_pubkey);
    assert_eq!(state.mint_a, accounts.mint_a);
    assert_eq!(state.mint_b, accounts.mint_b);
    assert_eq!(state.receive, RECEIVE);
    assert_eq!(state.bump, accounts.escrow_bump);

    // Vault received the deposit, maker ATA was debited.
    assert_eq!(token_amount(&svm, &accounts.vault), DEPOSIT);
    assert_eq!(
        token_amount(&svm, &accounts.maker_ata_a),
        MAKER_ATA_AMOUNT - DEPOSIT
    );
}

#[test]
fn test_make_zero_deposit_fails() {
    let MakeFixture { mut svm, accounts } = setup();

    let ix = make_tx(&accounts, SEED, 0, RECEIVE);
    let err = send(&mut svm, &[&accounts.maker], ix).unwrap_err();

    let expected = EscrowError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected InvalidAmount, got: {err_str}"
    );
}

#[test]
fn test_make_zero_receive_fails() {
    let MakeFixture { mut svm, accounts } = setup();

    let ix = make_tx(&accounts, SEED, DEPOSIT, 0);
    let err = send(&mut svm, &[&accounts.maker], ix).unwrap_err();

    let expected = EscrowError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected InvalidAmount, got: {err_str}"
    );
}
