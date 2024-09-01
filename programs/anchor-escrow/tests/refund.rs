mod helpers;

use {
    anchor_lang::prelude::*,
    helpers::{*, spl_token_helpers::*},
    rand::Rng,
    solana_program_test::*,
    solana_sdk::{
        account::Account as SolanaAccount,
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    },
    std::i64,
    anchor_escrow::Escrow,
};

// Testing the Refund instruction using the Bank for creating the Escrow account 
// and the spl_token_helpers for creating the mints and token accounts. At the end,
// the verification of the final states.

#[tokio::test]
async fn successful_token_refund() {
    let mut test = ProgramTest::new(
        "anchor_escrow",
        anchor_escrow::id(),
        None,
    );

    // Set compute unit limit
    test.set_compute_max_units(100_000);

    let maker = Keypair::new();
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let seed: u64 = rand::thread_rng().gen();

    let (escrow_pubkey, bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), seed.to_le_bytes().as_ref()],
        &anchor_escrow::id(),
    );

    let escrow = Escrow {
        seed,
        maker: maker.pubkey(),
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        receive: 100,
        expiry: i64::MAX,
        bump,
    };

    let mut escrow_data = Vec::with_capacity(Escrow::INIT_SPACE);
    escrow.try_serialize(&mut escrow_data).unwrap();

    test.add_account(
        escrow_pubkey,
        SolanaAccount {
            lamports: u32::MAX as u64,
            data: escrow_data,
            owner: anchor_escrow::id(),
            ..SolanaAccount::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Airdrop SOL to maker
    airdrop(&mut banks_client, &payer, &maker.pubkey(), LAMPORTS_PER_SOL).await.unwrap();
    
    // Create Mint A
    let mint_a = create_mint(&mut banks_client, &payer, Some(mint_a)).await.unwrap();

    // Verify Mint A
    get_mint(&mut banks_client, mint_a).await.expect("failed to get mint");

    // Create and mint tokens to vault
    let vault = create_and_mint_to_token_account(
        &mut banks_client,
        mint_a,
        &payer,
        escrow_pubkey,
        100_000,
    ).await.unwrap();

    // Verify vault
    get_token_account(&mut banks_client, vault).await.expect("failed to get token account");
    
    // Get initial balances
    let initial_vault_balance = get_token_balance(&mut banks_client, vault).await.unwrap();
    let maker_ata = spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);
    let initial_maker_balance = get_token_balance(&mut banks_client, maker_ata).await.unwrap_or(0);

    // Create and sign the refund transaction
    let mut transaction = Transaction::new_with_payer(
        &[refund(
            anchor_escrow::id(),
            spl_token::id(),
            maker.pubkey(),
            mint_a,
            escrow_pubkey,
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &maker], recent_blockhash);

    // Process the transaction
    banks_client.process_transaction(transaction).await.unwrap();

    // Verify final balances
    let final_maker_balance = get_token_balance(&mut banks_client, maker_ata).await.unwrap();

    assert_eq!(
        final_maker_balance,
        initial_maker_balance + initial_vault_balance,
        "Maker should receive the refunded tokens"
    );

    // Verify accounts are closed
    let vault_account = banks_client.get_account(vault).await.unwrap();
    assert!(vault_account.is_none(), "Vault should be closed");

    let escrow_account = banks_client.get_account(escrow_pubkey).await.unwrap();
    assert!(escrow_account.is_none(), "Escrow account should be closed after refund");
}