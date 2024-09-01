use anchor_lang::error_code;

#[error_code]
pub enum EscrowErrors {
    #[msg("Escrow Expired")]
    EscrowExpired,
}