pub mod spl_token_helpers;

use {
    anchor_lang::error::ERROR_CODE_OFFSET,
    solana_program_test::{BanksClient, BanksClientError, ProgramTestContext},
    solana_sdk::{
        clock::Clock,
        instruction::{Instruction, InstructionError},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        system_instruction::transfer,
        system_program,
        transaction::{Transaction, TransactionError},
    },
    spl_associated_token_account::get_associated_token_address_with_program_id,
    anchor_escrow::errors::EscrowErrors,
};

#[allow(dead_code)]
pub async fn airdrop(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    receiver: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let transaction = Transaction::new_signed_with_payer(
        &[transfer(&payer.pubkey(), receiver, amount)],
        Some(&payer.pubkey()),
        &[payer],
        banks_client.get_latest_blockhash().await?,
    );

    banks_client.process_transaction(transaction).await
}

#[allow(dead_code)]
pub async fn advance_clock(
    context: &mut ProgramTestContext, 
    seconds_to_advance: i64
) {
    let mut clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    clock.unix_timestamp += seconds_to_advance;
    context.set_sysvar(&clock);
}

#[allow(dead_code)]
pub fn assert_escrow_error(
    error: BanksClientError, 
    expected_error: EscrowErrors
) {
    if let BanksClientError::TransactionError(TransactionError::InstructionError(_, InstructionError::Custom(error_code))) = error {
        assert_eq!(
            error_code,
            expected_error as u32 + ERROR_CODE_OFFSET,
            "Expected error code {}, but got {}",
            expected_error as u32 + ERROR_CODE_OFFSET,
            error_code
        );
    } else {
        panic!("Expected InstructionError::Custom, but got {:?}", error);
    }
}

#[allow(dead_code)]
pub fn make(
    program_id: Pubkey,
    token_program_id: Pubkey,
    seed: u64,
    deposit: u64,
    receive: u64,
    expiry: u64,
    maker: Pubkey,
    mint_a: Pubkey,
    mint_b: Pubkey,
) -> Instruction {
    let maker_ata_a = get_associated_token_address_with_program_id(&maker, &mint_a, &token_program_id);
    let (escrow, _) = Pubkey::find_program_address(&[b"escrow", maker.as_ref(), seed.to_le_bytes().as_ref()], &program_id);
    let vault = get_associated_token_address_with_program_id(&escrow, &mint_a, &token_program_id);

    Instruction {
        program_id,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(
            &anchor_escrow::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program: spl_associated_token_account::id(),
                token_program: token_program_id,
                system_program: system_program::id(),
            },
            None,
        ),
        data: anchor_lang::InstructionData::data(
            &anchor_escrow::instruction::Make {
                seed,
                deposit,
                receive,
                expiry,
            },
        )
    }
}

#[allow(dead_code)]
pub fn take(
    program_id: Pubkey,
    token_program_id: Pubkey,
    taker: Pubkey,
    maker: Pubkey,
    mint_a: Pubkey,
    mint_b: Pubkey,
    escrow: Pubkey,
) -> Instruction {
    let taker_ata_a = get_associated_token_address_with_program_id(&taker, &mint_a, &token_program_id);
    let taker_ata_b = get_associated_token_address_with_program_id(&taker, &mint_b, &token_program_id);
    let maker_ata_b = get_associated_token_address_with_program_id(&maker, &mint_b, &token_program_id);
    let vault = get_associated_token_address_with_program_id(&escrow, &mint_a, &token_program_id);

    Instruction {
        program_id,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(
            &anchor_escrow::accounts::Take {
                taker,
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program: spl_associated_token_account::id(),
                token_program: token_program_id,
                system_program: system_program::id(),
            },
            None,
        ),
        data: anchor_lang::InstructionData::data(&anchor_escrow::instruction::Take {}),
    }
}

#[allow(dead_code)]
pub fn refund(
    program_id: Pubkey,
    token_program_id: Pubkey,
    maker: Pubkey,
    mint_a: Pubkey,
    escrow: Pubkey,
) -> Instruction {
    let maker_ata_a = get_associated_token_address_with_program_id(&maker, &mint_a, &token_program_id);
    let vault = get_associated_token_address_with_program_id(&escrow, &mint_a, &token_program_id);

    Instruction {
        program_id,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(
            &anchor_escrow::accounts::Refund {
                maker,
                mint_a,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program: spl_associated_token_account::id(),
                token_program: token_program_id,
                system_program: system_program::id(),
            },
            None,
        ),
        data: anchor_lang::InstructionData::data(&anchor_escrow::instruction::Refund {}),
    }
}