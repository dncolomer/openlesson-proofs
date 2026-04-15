use anchor_lang::prelude::*;

/// PDA seed for UserProofIndex accounts
#[constant]
pub const SEED_USER_INDEX: &[u8] = b"user_index";

/// PDA seed for ProofAnchor accounts
#[constant]
pub const SEED_PROOF: &[u8] = b"proof";

/// PDA seed for BatchAnchor accounts
#[constant]
pub const SEED_BATCH: &[u8] = b"batch";

/// Maximum valid proof type value (session_batch = 8)
pub const MAX_PROOF_TYPE: u8 = 8;

/// Maximum number of proofs in a single batch
pub const MAX_BATCH_SIZE: u32 = 1000;
