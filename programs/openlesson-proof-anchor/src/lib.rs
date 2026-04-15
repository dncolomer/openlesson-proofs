pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("6kFPmDutPLRigDcyKaLAtRnDbBZAh7teLfAMQLWhZJ5J");

#[program]
pub mod openlesson_proof_anchor {
    use super::*;

    /// Initialize a user's proof index account. Called once per user.
    /// The fee payer (OpenLesson) covers rent; the user signs to prove wallet ownership.
    pub fn initialize_user_account(
        ctx: Context<InitializeUserAccount>,
        user_id_hash: [u8; 32],
    ) -> Result<()> {
        instructions::initialize_user_account::handle_initialize_user_account(ctx, user_id_hash)
    }

    /// Anchor an individual proof fingerprint on-chain.
    /// Creates a ProofAnchor PDA and updates the user's proof index.
    pub fn anchor_proof(
        ctx: Context<AnchorProofCtx>,
        proof_id: [u8; 32],
        fingerprint: [u8; 32],
        proof_type: u8,
        user_id_hash: [u8; 32],
        event_timestamp: i64,
        session_id_hash: [u8; 32],
        plan_id_hash: [u8; 32],
    ) -> Result<()> {
        instructions::anchor_proof::handle_anchor_proof(
            ctx,
            proof_id,
            fingerprint,
            proof_type,
            user_id_hash,
            event_timestamp,
            session_id_hash,
            plan_id_hash,
        )
    }

    /// Anchor a batch of session proofs (Merkle root) on-chain.
    /// Creates a BatchAnchor PDA and updates the user's proof index.
    pub fn anchor_batch(
        ctx: Context<AnchorBatchCtx>,
        batch_id: [u8; 32],
        merkle_root: [u8; 32],
        proof_count: u32,
        user_id_hash: [u8; 32],
        session_id_hash: [u8; 32],
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<()> {
        instructions::anchor_batch::handle_anchor_batch(
            ctx,
            batch_id,
            merkle_root,
            proof_count,
            user_id_hash,
            session_id_hash,
            start_timestamp,
            end_timestamp,
        )
    }
}
