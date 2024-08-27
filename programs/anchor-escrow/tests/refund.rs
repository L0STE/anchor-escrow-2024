mod helpers;

use {
    helpers::*, 
    rand::Rng, 
    solana_program_test::*, 
    solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction},
    anchor_lang::prelude::*,
    solana_sdk::account::Account as SolanaAccount,
};

#[tokio::test]
async fn successful_token_refund() {
    let mut test = ProgramTest::new(
        "anchor_escrow",
        anchor_escrow::id(),
        None,
    );

    let maker = Keypair::new();
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let seed: u64 = rand::thread_rng().gen();

    let (escrow_pubkey, bump) = Pubkey::find_program_address(&[b"escrow", maker.pubkey().as_ref(), seed.to_le_bytes().as_ref()], &anchor_escrow::id());
    let escrow = anchor_escrow::Escrow {
        seed,
        maker: maker.pubkey(),
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        receive: 100,
        bump,
    };

    let mut escrow_data = Vec::with_capacity(anchor_escrow::Escrow::INIT_SPACE);
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

    // Airdrop SOL to taker
    airdrop(
        &mut banks_client, 
        &payer, 
        &maker.pubkey(), 
        1 * LAMPORTS_PER_SOL
    ).await;
    
    // Create Mint A and Mint B
    let mint_a = create_mint(&mut banks_client, &payer, Some(mint_a))
        .await;

    get_mint(&mut banks_client, mint_a)
        .await
        .expect("failed to get mint");

    // Create and mint tokens to vault and taker's account
    let vault = create_and_mint_to_token_account(
        &mut banks_client,
        mint_a,
        &payer,
        escrow_pubkey,
        100_000,
    ).await;

    get_token(&mut banks_client, vault)
        .await
        .expect("failed to get token");
    
    // Create and sign the take transaction
    let mut transaction = Transaction::new_with_payer(
        &[
            refund(
                anchor_escrow::id(), 
                spl_token::id(), 
                maker.pubkey(),
                mint_a,
                escrow_pubkey
            )
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &maker], recent_blockhash);

    // Process the transaction
    banks_client.process_transaction(transaction).await.unwrap();
}
