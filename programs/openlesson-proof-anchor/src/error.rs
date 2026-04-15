use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid proof type. Must be 0-8.")]
    InvalidProofType,

    #[msg("Batch cannot be empty. proof_count must be > 0.")]
    EmptyBatch,

    #[msg("Batch exceeds maximum size of 1000 proofs.")]
    BatchTooLarge,

    #[msg("Timestamps are invalid. end_timestamp must be >= start_timestamp.")]
    InvalidTimestamps,
}
