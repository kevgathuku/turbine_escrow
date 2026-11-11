mod common;

use {anchor_lang::AccountDeserialize, common::*, turbine_escrow::error::EscrowError};

const NEW_RECEIVE: u64 = 300 * TOKEN_UNIT;

#[test]
fn test_update_success() {
    let MakeFixture { mut svm, accounts } = setup();

    // Establish the escrow
    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    // The maker reprices the escrow - receive amount
    let update_ix = update_tx(&accounts, NEW_RECEIVE);
    send(&mut svm, &[&accounts.maker], update_ix).expect("update should succeed");

    // Only the receive amount changed; the vault is untouched.
    let acct = svm.get_account(&accounts.escrow).unwrap();
    let mut data: &[u8] = &acct.data;
    let state = turbine_escrow::state::Escrow::try_deserialize(&mut data).unwrap();
    assert_eq!(state.receive, NEW_RECEIVE);
    assert_eq!(token_amount(&svm, &accounts.vault), DEPOSIT);
    assert_eq!(
        token_amount(&svm, &accounts.maker_ata_a),
        MAKER_ATA_AMOUNT - DEPOSIT
    );
}

#[test]
fn test_update_zero_receive_fails() {
    let MakeFixture { mut svm, accounts } = setup();

    // Establish the escrow
    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    let update_ix = update_tx(&accounts, 0);
    let err = send(&mut svm, &[&accounts.maker], update_ix).unwrap_err();

    let expected = EscrowError::InvalidReceive.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected InvalidReceive, got: {err_str}"
    );
}
