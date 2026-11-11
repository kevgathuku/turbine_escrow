mod common;

use {anchor_lang::error::ErrorCode, common::*, solana_signer::Signer};

#[test]
fn test_refund_success() {
    let MakeFixture { mut svm, accounts } = setup();

    // Establish the escrow.
    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    // The maker cancels and gets the full deposit back.
    let ix = refund_tx(&accounts);
    send(&mut svm, &[&accounts.maker], ix).expect("refund should succeed");

    assert_eq!(token_amount(&svm, &accounts.maker_ata_a), MAKER_ATA_AMOUNT);
    assert!(svm.get_account(&accounts.escrow).is_none());
    assert!(svm.get_account(&accounts.vault).is_none());
}

#[test]
fn test_refund_stranger_fails() {
    let MakeFixture { mut svm, accounts } = setup();

    let ix = make_tx(&accounts, SEED, DEPOSIT, RECEIVE);
    send(&mut svm, &[&accounts.maker], ix).expect("make should succeed");

    let stranger = solana_keypair::Keypair::new();
    svm.airdrop(&stranger.pubkey(), LAMPORTS_PER_SOL).unwrap();
    let ix = refund_tx_with_maker(&accounts, &stranger.pubkey());
    let err = send(&mut svm, &[&stranger], ix).unwrap_err();

    // A stranger's key derives a different address and fails
    let expected = ErrorCode::ConstraintSeeds.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected ConstraintSeeds, got: {err_str}"
    );

    // Nothing moved: Vault still holds the full balance
    assert_eq!(token_amount(&svm, &accounts.vault), DEPOSIT);
}
