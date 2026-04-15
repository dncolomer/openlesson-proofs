use anchor_lang::prelude::*;

use crate::constants::SEED_USER_INDEX;
use crate::state::UserProofIndex;

#[derive(Accounts)]
pub struct InitializeUserAccount<'info> {
    /// The user whose proof index is being created.
    /// Must sign to prove ownership of the wallet.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Fee payer — OpenLesson's wallet that covers rent and transaction fees.
    /// Separate from the user so users never pay gas.
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// The UserProofIndex PDA to be created.
    /// Seeds: ["user_index", user.key()]
    #[account(
        init,
        payer = fee_payer,
        space = UserProofIndex::LEN,
        seeds = [SEED_USER_INDEX, user.key().as_ref()],
        bump,
    )]
    pub user_index: Account<'info, UserProofIndex>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_user_account(ctx: Context<InitializeUserAccount>, user_id_hash: [u8; 32]) -> Result<()> {
    let user_index = &mut ctx.accounts.user_index;
    let clock = Clock::get()?;

    user_index.user_pubkey = ctx.accounts.user.key();
    user_index.user_id_hash = user_id_hash;
    user_index.total_proofs = 0;
    user_index.total_batches = 0;
    user_index.total_heartbeats = 0;
    user_index.first_proof_timestamp = 0;
    user_index.last_proof_timestamp = 0;
    user_index.created_slot = clock.slot;
    user_index.bump = ctx.bumps.user_index;

    msg!(
        "Initialized user proof index for {}",
        ctx.accounts.user.key()
    );

    Ok(())
}
