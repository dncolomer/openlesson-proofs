use anchor_lang::prelude::*;

/// Per-user proof statistics. One account per user, derived from their public key.
/// PDA seeds: ["user_index", user_pubkey]
#[account]
pub struct UserProofIndex {
    /// User's Solana public key
    pub user_pubkey: Pubkey,
    /// SHA-256 hash of the OpenLesson user ID (UUID)
    pub user_id_hash: [u8; 32],
    /// Total individual proofs anchored
    pub total_proofs: u64,
    /// Total batch proofs anchored
    pub total_batches: u64,
    /// Sum of all proof_counts from batch anchors (total heartbeats)
    pub total_heartbeats: u64,
    /// Unix timestamp of the first proof
    pub first_proof_timestamp: i64,
    /// Unix timestamp of the most recent proof
    pub last_proof_timestamp: i64,
    /// Solana slot when this account was created
    pub created_slot: u64,
    /// PDA bump seed
    pub bump: u8,
}

impl UserProofIndex {
    /// Account size: 8 (discriminator) + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1 = 121 bytes
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1;
}

/// Individual proof fingerprint stored on-chain.
/// PDA seeds: ["proof", proof_id]
#[account]
pub struct ProofAnchor {
    /// Proof ID — SHA-256 of the UUID
    pub proof_id: [u8; 32],
    /// SHA-256 fingerprint of the proof event data
    pub fingerprint: [u8; 32],
    /// Proof type enum value (0-8)
    pub proof_type: u8,
    /// User's Solana public key
    pub user_pubkey: Pubkey,
    /// SHA-256 hash of the OpenLesson user ID (UUID)
    pub user_id_hash: [u8; 32],
    /// Unix timestamp of the original event
    pub event_timestamp: i64,
    /// Solana slot when this proof was anchored
    pub anchor_slot: u64,
    /// Unix timestamp when this proof was anchored
    pub anchor_timestamp: i64,
    /// SHA-256 hash of the related session ID (zeroed if none)
    pub session_id_hash: [u8; 32],
    /// SHA-256 hash of the related plan ID (zeroed if none)
    pub plan_id_hash: [u8; 32],
    /// PDA bump seed
    pub bump: u8,
}

impl ProofAnchor {
    /// Account size: 8 (discriminator) + 32 + 32 + 1 + 32 + 32 + 8 + 8 + 8 + 32 + 32 + 1 = 226 bytes
    pub const LEN: usize = 8 + 32 + 32 + 1 + 32 + 32 + 8 + 8 + 8 + 32 + 32 + 1;
}

/// Merkle root of a batch of session proofs (heartbeats + assistant queries).
/// PDA seeds: ["batch", batch_id]
#[account]
pub struct BatchAnchor {
    /// Batch ID — SHA-256 of the UUID
    pub batch_id: [u8; 32],
    /// Merkle root of all proof fingerprints in the batch
    pub merkle_root: [u8; 32],
    /// Number of proofs in the batch
    pub proof_count: u32,
    /// User's Solana public key
    pub user_pubkey: Pubkey,
    /// SHA-256 hash of the OpenLesson user ID (UUID)
    pub user_id_hash: [u8; 32],
    /// SHA-256 hash of the session ID this batch belongs to
    pub session_id_hash: [u8; 32],
    /// Unix timestamp of the first proof in the batch
    pub start_timestamp: i64,
    /// Unix timestamp of the last proof in the batch
    pub end_timestamp: i64,
    /// Solana slot when this batch was anchored
    pub anchor_slot: u64,
    /// Unix timestamp when this batch was anchored
    pub anchor_timestamp: i64,
    /// PDA bump seed
    pub bump: u8,
}

impl BatchAnchor {
    /// Account size: 8 (discriminator) + 32 + 32 + 4 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 = 205 bytes
    pub const LEN: usize = 8 + 32 + 32 + 4 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1;
}
