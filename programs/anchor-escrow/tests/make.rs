mod helpers;

use {
    helpers::*, 
    rand::Rng, 
    solana_program_test::*, 
    solana_sdk::{native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer, transaction::Transaction}
};

#[tokio::test]
async fn successful_token_make() {
    let test = ProgramTest::new(
        "anchor_escrow",
        anchor_escrow::id(),
        None,
    );

    let maker = Keypair::new();
    let seed: u64 = rand::thread_rng().gen();

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    airdrop(
        &mut banks_client, 
        &payer, 
        &maker.pubkey(), 
        1 * LAMPORTS_PER_SOL
    ).await;
    
    let mint_a = create_mint(
        &mut banks_client,
        &payer,
        None,
    ).await;

    get_mint(&mut banks_client, mint_a)
        .await
        .expect("failed to get mint");

    let mint_b = create_mint(
        &mut banks_client,
        &payer,
        None,
    ).await;

    get_mint(&mut banks_client, mint_b)
        .await
        .expect("failed to get mint");

    let maker_mint_a = create_and_mint_to_token_account(
        &mut banks_client,
        mint_a,
        &payer,
        maker.pubkey(),
        100_000,
    ).await;

    get_token(&mut banks_client, maker_mint_a)
        .await
        .expect("failed to get token");
    
    let mut transaction = Transaction::new_with_payer(
        &[
            make(
                anchor_escrow::id(), 
                spl_token::id(), 
                seed, 
                100, 
                100, 
                maker.pubkey(), 
                mint_a, 
                mint_b
            )
        ],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &maker], recent_blockhash);

    // banks_client.process_transaction_with_preflight(transaction).await;
    banks_client.process_transaction(transaction).await.unwrap();
}