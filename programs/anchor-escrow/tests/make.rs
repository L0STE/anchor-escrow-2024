mod helpers;

use {
    anchor_escrow::state::Escrow, anchor_lang::AccountDeserialize, helpers::{spl_token_helpers::*, *}, rand::Rng, solana_program_test::*, solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction}, std::u64
};

// Testing the Make instruction using the spl_token_helpers and creating
// mints and token accounts normally. Adding at the end the verification of 
// the escrow state.

#[tokio::test]
async fn test_successful_make() {
    let mut test = ProgramTest::new(
        "anchor_escrow",
        anchor_escrow::id(),
        None,
    );

    // Set compute unit limit
    test.set_compute_max_units(100_000);

    let maker = Keypair::new();
    let seed: u64 = rand::thread_rng().gen();

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Airdrop SOL to maker
    let _ = airdrop(&mut banks_client, &payer, &maker.pubkey(), 2 * LAMPORTS_PER_SOL).await;

    // Create mints
    let mint_a = create_mint(&mut banks_client, &payer, None).await.unwrap();
    let mint_b = create_mint(&mut banks_client, &payer, None).await.unwrap();

    // Create and mint tokens to maker's account
    let _ = create_and_mint_to_token_account(&mut banks_client, mint_a, &payer, maker.pubkey(), 100_000).await;

    let mut transaction = Transaction::new_with_payer(
        &[make(
            anchor_escrow::id(),
            spl_token::id(),
            seed,
            100,
            100,
            u64::MAX,
            maker.pubkey(),
            mint_a,
            mint_b,
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &maker], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Verify escrow state
    let (escrow_pubkey, _) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), seed.to_le_bytes().as_ref()],
        &anchor_escrow::id(),
    );

    let escrow = banks_client
        .get_account(escrow_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Deserialize the account data
    let mut account_data = escrow.data.as_ref();
    let escrow_account = Escrow::try_deserialize(&mut account_data).unwrap();

    assert_eq!(escrow_account.maker, maker.pubkey());
    assert_eq!(escrow_account.mint_a, mint_a);
    assert_eq!(escrow_account.mint_b, mint_b);
    assert_eq!(escrow_account.receive, 100);

}