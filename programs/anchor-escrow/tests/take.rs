mod helpers;

use {
    anchor_escrow::state::Escrow, anchor_lang::prelude::*, helpers::{*, spl_token_helpers::*}, rand::Rng, solana_program::program_pack::Pack, solana_program_test::*, solana_sdk::{
        account::{Account as SolanaAccount, AccountSharedData},
        native_token::LAMPORTS_PER_SOL,
        program_option::COption,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    }, spl_token::state::{Account as TokenAccount, AccountState, Mint}
};

// Testing the Take instruction using the Bank for creating all the accounts
// needed for the test. At the end, the verification of the final states.

#[tokio::test]
async fn successful_take() {
    let mut test = ProgramTest::new(
        "anchor_escrow",
        anchor_escrow::id(),
        None,
    );

    // Set compute unit limit
    test.set_compute_max_units(200_000);

    let maker = Keypair::new();
    let taker = Keypair::new();
    let mint_a = Keypair::new().pubkey();
    let mint_b = Keypair::new().pubkey();
    let seed: u64 = rand::thread_rng().gen();
    let (escrow_pubkey, bump) = Pubkey::find_program_address(&[b"escrow", maker.pubkey().as_ref(), seed.to_le_bytes().as_ref()], &anchor_escrow::id());

    // Setup escrow account
    let mut escrow_data = vec![];
    let escrow = Escrow {
        seed,
        maker: maker.pubkey(),
        mint_a,
        mint_b,
        receive: 100,
        expiry: i64::MAX,
        bump,
    };
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

    // Setup mint accounts
    for (mint, supply) in [(mint_a, 100_000), (mint_b, 100_000)] {
        let mut mint_data = vec![0u8; Mint::LEN];
        Mint {
            is_initialized: true,
            decimals: 6,
            mint_authority: COption::None,
            supply,
            ..Mint::default()
        }.pack_into_slice(&mut mint_data);
        test.add_account(
            mint,
            SolanaAccount {
                lamports: u32::MAX as u64,
                data: mint_data,
                owner: spl_token::id(),
                ..SolanaAccount::default()
            },
        );
    }

    // Setup token accounts
    let vault = spl_associated_token_account::get_associated_token_address(&escrow_pubkey, &mint_a);
    let taker_mint_b = spl_associated_token_account::get_associated_token_address(&taker.pubkey(), &mint_b);
    let maker_mint_a = spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);

    for (account, mint, owner, amount) in [
        (taker_mint_b, mint_b, taker.pubkey(), 100_000),
        (vault, mint_a, escrow_pubkey, 100),
        (maker_mint_a, mint_a, maker.pubkey(), 0),
    ] {
        let mut account_data = vec![0u8; TokenAccount::LEN];
        TokenAccount {
            mint,
            owner,
            amount,
            state: AccountState::Initialized,
            ..TokenAccount::default()
        }.pack_into_slice(&mut account_data);
        test.add_account(
            account,
            SolanaAccount {
                lamports: u32::MAX as u64,
                data: account_data,
                owner: spl_token::id(),
                ..SolanaAccount::default()
            },
        );
    }

    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Airdrop SOL to taker
    let _ = airdrop(&mut banks_client, &payer, &taker.pubkey(), 1 * LAMPORTS_PER_SOL).await;

    // Execute take instruction
    let mut transaction = Transaction::new_with_payer(
        &[take(
            anchor_escrow::id(),
            spl_token::id(),
            taker.pubkey(),
            maker.pubkey(),
            mint_a,
            mint_b,
            escrow_pubkey,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &taker], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Verify final states
    let vault_account = banks_client.get_account(vault).await.unwrap();
    assert!(vault_account.is_none(), "Vault should be closed");

    let taker_mint_a = spl_associated_token_account::get_associated_token_address(&taker.pubkey(), &mint_a);
    let taker_mint_a_balance = get_token_balance(&mut banks_client, taker_mint_a).await.unwrap();
    assert_eq!(taker_mint_a_balance, 100, "Taker should receive 100 tokens of mint A");

    let maker_mint_b = spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_b);
    let maker_mint_b_balance = get_token_balance(&mut banks_client, maker_mint_b).await.unwrap();
    assert_eq!(maker_mint_b_balance, 100, "Maker should receive 100 tokens of mint B");

    let escrow_account = banks_client.get_account(escrow_pubkey).await.unwrap();
    assert!(escrow_account.is_none(), "Escrow account should be closed");
}

// Testing the Take instruction using the Bank for creating all the accounts
// needed for the test and creating the Escrow State leveraging the context. 
// At the end, the verification of the final states.

#[tokio::test]
async fn take_error_escrow_expired() {
    let mut test = ProgramTest::new(
        "anchor_escrow",
        anchor_escrow::id(),
        None,
    );

    // Set compute unit limit
    test.set_compute_max_units(100_000);

    let maker = Keypair::new();
    let taker = Keypair::new();
    let mint_a = Keypair::new().pubkey();
    let mint_b = Keypair::new().pubkey();
    let seed: u64 = rand::thread_rng().gen();
    let (escrow_pubkey, bump) = Pubkey::find_program_address(&[b"escrow", maker.pubkey().as_ref(), seed.to_le_bytes().as_ref()], &anchor_escrow::id());
    let vault = spl_associated_token_account::get_associated_token_address(&escrow_pubkey, &mint_a);
    let taker_mint_b = spl_associated_token_account::get_associated_token_address(&taker.pubkey(), &mint_b);
    let maker_mint_a = spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);

    // Setup mint accounts
    for (mint, supply) in [(mint_a, 100_000), (mint_b, 100_000)] {
        let mut mint_data = vec![0u8; Mint::LEN];
        Mint {
            is_initialized: true,
            decimals: 6,
            mint_authority: COption::None,
            supply,
            ..Mint::default()
        }.pack_into_slice(&mut mint_data);
        test.add_account(
            mint,
            SolanaAccount {
                lamports: u32::MAX as u64,
                data: mint_data,
                owner: spl_token::id(),
                ..SolanaAccount::default()
            },
        );
    }

    // Setup token accounts
    for (account, mint, owner, amount) in [
        (taker_mint_b, mint_b, taker.pubkey(), 100_000),
        (vault, mint_a, escrow_pubkey, 100),
        (maker_mint_a, mint_a, maker.pubkey(), 0),
    ] {
        let mut account_data = vec![0u8; TokenAccount::LEN];
        TokenAccount {
            mint,
            owner,
            amount,
            state: AccountState::Initialized,
            ..TokenAccount::default()
        }.pack_into_slice(&mut account_data);
        test.add_account(
            account,
            SolanaAccount {
                lamports: u32::MAX as u64,
                data: account_data,
                owner: spl_token::id(),
                ..SolanaAccount::default()
            },
        );
    }

    let mut context = test.start_with_context().await;

    // Airdrop SOL to taker
    let _ = airdrop(&mut context.banks_client, &context.payer, &taker.pubkey(), 1 * LAMPORTS_PER_SOL).await;

    // Get the current timestamp
    let clock = context.banks_client.get_sysvar::<Clock>().await.unwrap();
    let current_time = clock.unix_timestamp;

    // Setup escrow account with current time as expiry
    let escrow = Escrow {
        seed,
        maker: maker.pubkey(),
        mint_a,
        mint_b,
        receive: 100,
        expiry: current_time,
        bump,
    };

    let mut escrow_data = vec![0u8; Escrow::INIT_SPACE];
    escrow.try_serialize(&mut escrow_data).unwrap();

    let mut account = AccountSharedData::new(
        u32::MAX as u64,
        Escrow::INIT_SPACE,
        &anchor_escrow::id(),
    );
    account.set_data_from_slice(&escrow_data);

    context.set_account(&escrow_pubkey, &account);

    // Move 60s into the future
    advance_clock(&mut context, 60).await;

    // Execute take instruction
    let mut transaction = Transaction::new_with_payer(
        &[take(
            anchor_escrow::id(),
            spl_token::id(),
            taker.pubkey(),
            maker.pubkey(),
            mint_a,
            mint_b,
            escrow_pubkey,
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &taker], context.last_blockhash);
    
    // Process the transaction and expect an error
    let result = context.banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Transaction should fail due to expired escrow");

    // Verify that the escrow and vault accounts still exist
    let escrow_account = context.banks_client.get_account(escrow_pubkey).await.unwrap();
    assert!(escrow_account.is_some(), "Escrow account should still exist");

    let vault_account = context.banks_client.get_account(vault).await.unwrap();
    assert!(vault_account.is_some(), "Vault account should still exist");
}