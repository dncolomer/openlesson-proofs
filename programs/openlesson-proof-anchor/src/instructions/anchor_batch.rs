use anchor_lang::prelude::*;

use crate::constants::{MAX_BATCH_SIZE, SEED_BATCH, SEED_USER_INDEX};
use crate::error::ErrorCode;
use crate::events::BatchAnchored;
use crate::state::{BatchAnchor, UserProofIndex};

#[derive(Accounts)]
#[instruction(batch_id: [u8; 32])]
pub struct AnchorBatchCtx<'info> {
    /// The user who owns this batch. Must sign to prove wallet ownership.
    pub user: Signer<'info>,

    /// Fee payer — OpenLesson's wallet that covers rent and transaction fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// The user's proof index PDA. Must already be initialized.
    /// Seeds: ["user_index", user.key()]
    #[account(
        mut,
        seeds = [SEED_USER_INDEX, user.key().as_ref()],
        bump = user_index.bump,
        constraint = user_index.user_pubkey == user.key(),
    )]
    pub user_index: Account<'info, UserProofIndex>,

    /// The BatchAnchor PDA to be created.
    /// Seeds: ["batch", batch_id]
    #[account(
        init,
        payer = fee_payer,
        space = BatchAnchor::LEN,
        seeds = [SEED_BATCH, batch_id.as_ref()],
        bump,
    )]
    pub batch_anchor: Account<'info, BatchAnchor>,

    pub system_program: Program<'info, System>,
}

pub fn handle_anchor_batch(
    ctx: Context<AnchorBatchCtx>,
    batch_id: [u8; 32],
    merkle_root: [u8; 32],
    proof_count: u32,
    user_id_hash: [u8; 32],
    session_id_hash: [u8; 32],
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<()> {
    // Validate batch is not empty
    require!(proof_count > 0, ErrorCode::EmptyBatch);

    // Validate batch size
    require!(proof_count <= MAX_BATCH_SIZE, ErrorCode::BatchTooLarge);

    // Validate timestamps
    require!(
        end_timestamp >= start_timestamp,
        ErrorCode::InvalidTimestamps
    );

    let clock = Clock::get()?;

    // Populate the batch anchor account
    let batch_anchor = &mut ctx.accounts.batch_anchor;
    batch_anchor.batch_id = batch_id;
    batch_anchor.merkle_root = merkle_root;
    batch_anchor.proof_count = proof_count;
    batch_anchor.user_pubkey = ctx.accounts.user.key();
    batch_anchor.user_id_hash = user_id_hash;
    batch_anchor.session_id_hash = session_id_hash;
    batch_anchor.start_timestamp = start_timestamp;
    batch_anchor.end_timestamp = end_timestamp;
    batch_anchor.anchor_slot = clock.slot;
    batch_anchor.anchor_timestamp = clock.unix_timestamp;
    batch_anchor.bump = ctx.bumps.batch_anchor;

    // Update user index
    let user_index = &mut ctx.accounts.user_index;
    user_index.total_batches = user_index.total_batches.saturating_add(1);
    user_index.total_heartbeats = user_index
        .total_heartbeats
        .saturating_add(proof_count as u64);

    if user_index.first_proof_timestamp == 0 {
        user_index.first_proof_timestamp = start_timestamp;
    }
    if end_timestamp > user_index.last_proof_timestamp {
        user_index.last_proof_timestamp = end_timestamp;
    }

    // Emit event for off-chain indexing
    emit!(BatchAnchored {
        batch_id,
        merkle_root,
        proof_count,
        user_pubkey: ctx.accounts.user.key(),
        session_id_hash,
        anchor_slot: clock.slot,
    });

    msg!("Batch anchored: {} proofs", proof_count);

    Ok(())
}
