use anchor_lang::prelude::*;

/// Emitted when an individual proof is anchored on-chain.
#[event]
pub struct ProofAnchored {
    pub proof_id: [u8; 32],
    pub fingerprint: [u8; 32],
    pub proof_type: u8,
    pub user_pubkey: Pubkey,
    pub event_timestamp: i64,
    pub anchor_slot: u64,
}

/// Emitted when a session batch (Merkle root) is anchored on-chain.
#[event]
pub struct BatchAnchored {
    pub batch_id: [u8; 32],
    pub merkle_root: [u8; 32],
    pub proof_count: u32,
    pub user_pubkey: Pubkey,
    pub session_id_hash: [u8; 32],
    pub anchor_slot: u64,
}
