pub mod spl_token_helpers;
pub use spl_token_helpers::*;

use {
    solana_program_test::BanksClient, 
    solana_sdk::{
        instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction::transfer, system_program, transaction::Transaction
    }, 
    spl_associated_token_account::get_associated_token_address_with_program_id, 
};

pub async fn airdrop(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    receiver: &Pubkey,
    amount: u64,
) {

    let mut transaction = Transaction::new_with_payer(
        &[
            transfer(
                &payer.pubkey(),
                receiver,
                amount,
            )
        ],
        Some(&payer.pubkey()),
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[payer], recent_blockhash);

    banks_client.process_transaction(transaction)
        .await
        .unwrap();
}

pub fn make(
    program_id: Pubkey,
    token_program_id: Pubkey,
    seed: u64,
    deposit: u64,
    receive: u64,
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
            },
        )
    }
}

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
        data: anchor_lang::InstructionData::data(
            &anchor_escrow::instruction::Take {
                // It's empty
            },
        )
    }
}

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
        data: anchor_lang::InstructionData::data(
            &anchor_escrow::instruction::Refund {
                // It's empty
            },
        )
    }
}

