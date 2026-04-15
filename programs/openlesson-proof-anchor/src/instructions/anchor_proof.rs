use anchor_lang::prelude::*;

use crate::constants::{MAX_PROOF_TYPE, SEED_PROOF, SEED_USER_INDEX};
use crate::error::ErrorCode;
use crate::events::ProofAnchored;
use crate::state::{ProofAnchor, UserProofIndex};

#[derive(Accounts)]
#[instruction(proof_id: [u8; 32])]
pub struct AnchorProofCtx<'info> {
    /// The user who owns this proof. Must sign to prove wallet ownership.
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

    /// The ProofAnchor PDA to be created.
    /// Seeds: ["proof", proof_id]
    #[account(
        init,
        payer = fee_payer,
        space = ProofAnchor::LEN,
        seeds = [SEED_PROOF, proof_id.as_ref()],
        bump,
    )]
    pub proof_anchor: Account<'info, ProofAnchor>,

    pub system_program: Program<'info, System>,
}

pub fn handle_anchor_proof(
    ctx: Context<AnchorProofCtx>,
    proof_id: [u8; 32],
    fingerprint: [u8; 32],
    proof_type: u8,
    user_id_hash: [u8; 32],
    event_timestamp: i64,
    session_id_hash: [u8; 32],
    plan_id_hash: [u8; 32],
) -> Result<()> {
    // Validate proof type is in range 0..=8
    require!(proof_type <= MAX_PROOF_TYPE, ErrorCode::InvalidProofType);

    let clock = Clock::get()?;

    // Populate the proof anchor account
    let proof_anchor = &mut ctx.accounts.proof_anchor;
    proof_anchor.proof_id = proof_id;
    proof_anchor.fingerprint = fingerprint;
    proof_anchor.proof_type = proof_type;
    proof_anchor.user_pubkey = ctx.accounts.user.key();
    proof_anchor.user_id_hash = user_id_hash;
    proof_anchor.event_timestamp = event_timestamp;
    proof_anchor.anchor_slot = clock.slot;
    proof_anchor.anchor_timestamp = clock.unix_timestamp;
    proof_anchor.session_id_hash = session_id_hash;
    proof_anchor.plan_id_hash = plan_id_hash;
    proof_anchor.bump = ctx.bumps.proof_anchor;

    // Update user index
    let user_index = &mut ctx.accounts.user_index;
    user_index.total_proofs = user_index.total_proofs.saturating_add(1);

    if user_index.first_proof_timestamp == 0 {
        user_index.first_proof_timestamp = event_timestamp;
    }
    if event_timestamp > user_index.last_proof_timestamp {
        user_index.last_proof_timestamp = event_timestamp;
    }

    // Emit event for off-chain indexing
    emit!(ProofAnchored {
        proof_id,
        fingerprint,
        proof_type,
        user_pubkey: ctx.accounts.user.key(),
        event_timestamp,
        anchor_slot: clock.slot,
    });

    msg!("Proof anchored: type={}", proof_type);

    Ok(())
}
