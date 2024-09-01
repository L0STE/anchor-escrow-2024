use {
    solana_program_test::{BanksClient, BanksClientError},
    solana_sdk::{
        program_pack::Pack,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        system_instruction::create_account,
        transaction::Transaction,
        transport::TransportError,
    },
    spl_token::state::{Account as TokenAccount, Mint},
};

#[allow(dead_code)]
pub async fn create_and_mint_to_token_account(
    banks_client: &mut BanksClient,
    mint_pubkey: Pubkey,
    payer: &Keypair,
    owner: Pubkey,
    amount: u64,
) -> Result<Pubkey, TransportError> {
    let account_pubkey = create_token_account(banks_client, mint_pubkey, payer, owner).await?;
    mint_to(banks_client, mint_pubkey, payer, account_pubkey, amount).await?;
    Ok(account_pubkey)
}

#[allow(dead_code)]
pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    provided_mint: Option<Keypair>,
) -> Result<Pubkey, TransportError> {
    let mint = provided_mint.unwrap_or_else(Keypair::new);
    let rent = banks_client.get_rent().await?;
    let lamports = rent.minimum_balance(Mint::LEN);

    let transaction = Transaction::new_signed_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                lamports,
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint2(
                &spl_token::id(),
                &mint.pubkey(),
                &payer.pubkey(),
                None,
                6,
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, &mint],
        banks_client.get_latest_blockhash().await?,
    );

    banks_client.process_transaction(transaction).await?;

    Ok(mint.pubkey())
}

#[allow(dead_code)]
pub async fn create_token_account(
    banks_client: &mut BanksClient,
    mint_pubkey: Pubkey,
    payer: &Keypair,
    owner: Pubkey,
) -> Result<Pubkey, TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &owner,
            &mint_pubkey,
            &spl_token::id(),
        )],
        Some(&payer.pubkey()),
        &[payer],
        banks_client.get_latest_blockhash().await?,
    );

    banks_client.process_transaction(transaction).await?;

    Ok(spl_associated_token_account::get_associated_token_address(&owner, &mint_pubkey))
}

#[allow(dead_code)]
pub async fn mint_to(
    banks_client: &mut BanksClient,
    mint_pubkey: Pubkey,
    payer: &Keypair,
    account_pubkey: Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint_pubkey,
            &account_pubkey,
            &payer.pubkey(),
            &[],
            amount,
        ).unwrap()],
        Some(&payer.pubkey()),
        &[payer],
        banks_client.get_latest_blockhash().await?,
    );

    banks_client.process_transaction(transaction).await
}

#[allow(dead_code)]
pub async fn get_mint(banks_client: &mut BanksClient, pubkey: Pubkey) -> Result<Mint, TransportError> {
    let account = banks_client
        .get_account(pubkey)
        .await?
        .ok_or_else(|| TransportError::Custom("Mint account not found".to_string()))?;
    Mint::unpack(&account.data).map_err(|err| TransportError::Custom(format!("Failed to unpack Mint: {:?}", err)))
}

#[allow(dead_code)]
pub async fn get_token_account(banks_client: &mut BanksClient, pubkey: Pubkey) -> Result<TokenAccount, TransportError> {
    let account = banks_client
        .get_account(pubkey)
        .await?
        .ok_or_else(|| TransportError::Custom("Token account not found".to_string()))?;
    TokenAccount::unpack(&account.data).map_err(|err| TransportError::Custom(format!("Failed to unpack Token account: {:?}", err)))
}

#[allow(dead_code)]
pub async fn get_token_balance(banks_client: &mut BanksClient, pubkey: Pubkey) -> Result<u64, TransportError> {
    let token_account = get_token_account(banks_client, pubkey).await?;
    Ok(token_account.amount)
}