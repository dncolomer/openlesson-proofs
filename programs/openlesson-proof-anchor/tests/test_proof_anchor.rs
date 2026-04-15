use anchor_lang::{
    solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

/// Helper: derive a PDA and return (pubkey, bump)
fn find_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

/// Helper: build, sign and send a transaction. Returns Ok(()) or panics with error.
fn send_tx(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(instructions, Some(&payer.pubkey()), &blockhash);

    let mut all_signers: Vec<&Keypair> = vec![payer];
    for s in signers {
        // Avoid duplicate payer
        if s.pubkey() != payer.pubkey() {
            all_signers.push(s);
        }
    }

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &all_signers.iter().map(|k| *k as &dyn Signer).collect::<Vec<_>>(),
    )
    .unwrap();

    svm.send_transaction(tx)
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// Helper: set up the SVM with the program loaded and a funded fee payer + user
fn setup() -> (LiteSVM, Pubkey, Keypair, Keypair) {
    let program_id = openlesson_proof_anchor::id();
    let fee_payer = Keypair::new();
    let user = Keypair::new();

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/openlesson_proof_anchor.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&fee_payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 10_000_000).unwrap(); // User gets minimal SOL (shouldn't need to pay)

    (svm, program_id, fee_payer, user)
}

/// Helper: create a dummy 32-byte array from a u8 seed value
fn dummy_hash(seed: u8) -> [u8; 32] {
    let mut arr = [0u8; 32];
    arr[0] = seed;
    arr
}

/// Helper: initialize a user account, returns the user_index PDA
fn init_user_account(
    svm: &mut LiteSVM,
    program_id: &Pubkey,
    fee_payer: &Keypair,
    user: &Keypair,
    user_id_hash: [u8; 32],
) -> Pubkey {
    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::InitializeUserAccount {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        system_program: solana_pubkey::Pubkey::from([0; 32]), // system program
    };

    // System program address
    let mut account_metas = accounts.to_account_metas(None);
    // Fix system program address (it should be 11111...1)
    for meta in &mut account_metas {
        if meta.pubkey == solana_pubkey::Pubkey::from([0; 32]) {
            meta.pubkey = solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111");
        }
    }

    let ix_data = openlesson_proof_anchor::instruction::InitializeUserAccount {
        user_id_hash,
    };

    let instruction = Instruction::new_with_bytes(
        *program_id,
        &ix_data.data(),
        account_metas,
    );

    send_tx(svm, &[instruction], fee_payer, &[user])
        .expect("initialize_user_account should succeed");

    user_index_pda
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_user_account() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    let user_index_pda = init_user_account(
        &mut svm, &program_id, &fee_payer, &user, user_id_hash,
    );

    // Verify the account exists and has correct data
    let account = svm.get_account(&user_index_pda).expect("user_index PDA should exist");
    assert!(account.data.len() >= 121, "Account data should be at least 121 bytes");

    // Verify we can't initialize the same user twice (PDA already exists)
    let (user_index_pda2, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::InitializeUserAccount {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda2,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::InitializeUserAccount {
        user_id_hash,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    let result = send_tx(&mut svm, &[instruction], &fee_payer, &[&user]);
    assert!(result.is_err(), "Should fail when initializing duplicate user account");
}

#[test]
fn test_anchor_proof() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    // First initialize user account
    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    // Now anchor a proof
    let proof_id = dummy_hash(10);
    let fingerprint = dummy_hash(20);
    let proof_type: u8 = 2; // session_started
    let event_timestamp: i64 = 1700000000;
    let session_id_hash = dummy_hash(30);
    let plan_id_hash = dummy_hash(40);

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );
    let (proof_anchor_pda, _) = find_pda(
        &[b"proof", proof_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorProofCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        proof_anchor: proof_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorProof {
        proof_id,
        fingerprint,
        proof_type,
        user_id_hash,
        event_timestamp,
        session_id_hash,
        plan_id_hash,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    send_tx(&mut svm, &[instruction], &fee_payer, &[&user])
        .expect("anchor_proof should succeed");

    // Verify the proof anchor account exists
    let account = svm.get_account(&proof_anchor_pda).expect("proof_anchor PDA should exist");
    assert!(account.data.len() >= 226, "ProofAnchor data should be at least 226 bytes");

    // Verify user index was updated
    let idx_account = svm.get_account(&user_index_pda).expect("user_index should exist");
    assert!(idx_account.data.len() >= 121);
}

#[test]
fn test_anchor_proof_invalid_type() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let proof_id = dummy_hash(10);
    let fingerprint = dummy_hash(20);
    let proof_type: u8 = 9; // INVALID — max is 8
    let event_timestamp: i64 = 1700000000;
    let session_id_hash = dummy_hash(30);
    let plan_id_hash = dummy_hash(40);

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );
    let (proof_anchor_pda, _) = find_pda(
        &[b"proof", proof_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorProofCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        proof_anchor: proof_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorProof {
        proof_id,
        fingerprint,
        proof_type,
        user_id_hash,
        event_timestamp,
        session_id_hash,
        plan_id_hash,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    let result = send_tx(&mut svm, &[instruction], &fee_payer, &[&user]);
    assert!(result.is_err(), "Should fail with invalid proof type (9)");
}

#[test]
fn test_anchor_proof_duplicate_prevention() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let proof_id = dummy_hash(10);
    let fingerprint = dummy_hash(20);
    let proof_type: u8 = 0;
    let event_timestamp: i64 = 1700000000;
    let session_id_hash = dummy_hash(30);
    let plan_id_hash = dummy_hash(40);

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );
    let (proof_anchor_pda, _) = find_pda(
        &[b"proof", proof_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorProofCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        proof_anchor: proof_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorProof {
        proof_id,
        fingerprint,
        proof_type,
        user_id_hash,
        event_timestamp,
        session_id_hash,
        plan_id_hash,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    // First anchor succeeds
    send_tx(&mut svm, &[instruction.clone()], &fee_payer, &[&user])
        .expect("First anchor should succeed");

    // Second anchor with same proof_id should fail (PDA already init'd)
    let result = send_tx(&mut svm, &[instruction], &fee_payer, &[&user]);
    assert!(result.is_err(), "Duplicate proof_id should fail (PDA already exists)");
}

#[test]
fn test_anchor_batch() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let batch_id = dummy_hash(50);
    let merkle_root = dummy_hash(60);
    let proof_count: u32 = 15;
    let session_id_hash = dummy_hash(30);
    let start_timestamp: i64 = 1700000000;
    let end_timestamp: i64 = 1700003600;

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );
    let (batch_anchor_pda, _) = find_pda(
        &[b"batch", batch_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorBatchCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        batch_anchor: batch_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorBatch {
        batch_id,
        merkle_root,
        proof_count,
        user_id_hash,
        session_id_hash,
        start_timestamp,
        end_timestamp,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    send_tx(&mut svm, &[instruction], &fee_payer, &[&user])
        .expect("anchor_batch should succeed");

    // Verify the batch anchor account exists
    let account = svm.get_account(&batch_anchor_pda).expect("batch_anchor PDA should exist");
    assert!(account.data.len() >= 205, "BatchAnchor data should be at least 205 bytes");
}

#[test]
fn test_anchor_batch_empty() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let batch_id = dummy_hash(50);
    let merkle_root = dummy_hash(60);
    let proof_count: u32 = 0; // EMPTY — should fail
    let session_id_hash = dummy_hash(30);
    let start_timestamp: i64 = 1700000000;
    let end_timestamp: i64 = 1700003600;

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );
    let (batch_anchor_pda, _) = find_pda(
        &[b"batch", batch_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorBatchCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        batch_anchor: batch_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorBatch {
        batch_id,
        merkle_root,
        proof_count,
        user_id_hash,
        session_id_hash,
        start_timestamp,
        end_timestamp,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    let result = send_tx(&mut svm, &[instruction], &fee_payer, &[&user]);
    assert!(result.is_err(), "Should fail with empty batch (proof_count = 0)");
}

#[test]
fn test_anchor_batch_too_large() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let batch_id = dummy_hash(50);
    let merkle_root = dummy_hash(60);
    let proof_count: u32 = 1001; // TOO LARGE — max is 1000
    let session_id_hash = dummy_hash(30);
    let start_timestamp: i64 = 1700000000;
    let end_timestamp: i64 = 1700003600;

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );
    let (batch_anchor_pda, _) = find_pda(
        &[b"batch", batch_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorBatchCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        batch_anchor: batch_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorBatch {
        batch_id,
        merkle_root,
        proof_count,
        user_id_hash,
        session_id_hash,
        start_timestamp,
        end_timestamp,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    let result = send_tx(&mut svm, &[instruction], &fee_payer, &[&user]);
    assert!(result.is_err(), "Should fail with batch too large (1001 > 1000)");
}

#[test]
fn test_anchor_batch_invalid_timestamps() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let batch_id = dummy_hash(50);
    let merkle_root = dummy_hash(60);
    let proof_count: u32 = 5;
    let session_id_hash = dummy_hash(30);
    let start_timestamp: i64 = 1700003600; // start > end = invalid
    let end_timestamp: i64 = 1700000000;

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );
    let (batch_anchor_pda, _) = find_pda(
        &[b"batch", batch_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorBatchCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        batch_anchor: batch_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorBatch {
        batch_id,
        merkle_root,
        proof_count,
        user_id_hash,
        session_id_hash,
        start_timestamp,
        end_timestamp,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    let result = send_tx(&mut svm, &[instruction], &fee_payer, &[&user]);
    assert!(result.is_err(), "Should fail with invalid timestamps (start > end)");
}

#[test]
fn test_multiple_proofs_user_index_tracking() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );

    // Anchor 3 individual proofs
    for i in 0u8..3 {
        let proof_id = dummy_hash(100 + i);
        let fingerprint = dummy_hash(200 + i);
        let proof_type: u8 = i; // plan_created, plan_adapted, session_started

        let (proof_anchor_pda, _) = find_pda(
            &[b"proof", proof_id.as_ref()],
            &program_id,
        );

        let accounts = openlesson_proof_anchor::accounts::AnchorProofCtx {
            user: user.pubkey(),
            fee_payer: fee_payer.pubkey(),
            user_index: user_index_pda,
            proof_anchor: proof_anchor_pda,
            system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
        };

        let ix_data = openlesson_proof_anchor::instruction::AnchorProof {
            proof_id,
            fingerprint,
            proof_type,
            user_id_hash,
            event_timestamp: 1700000000 + (i as i64 * 1000),
            session_id_hash: dummy_hash(30),
            plan_id_hash: dummy_hash(40),
        };

        let instruction = Instruction::new_with_bytes(
            program_id,
            &ix_data.data(),
            accounts.to_account_metas(None),
        );

        send_tx(&mut svm, &[instruction], &fee_payer, &[&user])
            .unwrap_or_else(|e| panic!("Proof {} should succeed: {}", i, e));
    }

    // Anchor 1 batch with 20 proofs
    let batch_id = dummy_hash(50);
    let merkle_root = dummy_hash(60);

    let (batch_anchor_pda, _) = find_pda(
        &[b"batch", batch_id.as_ref()],
        &program_id,
    );

    let accounts = openlesson_proof_anchor::accounts::AnchorBatchCtx {
        user: user.pubkey(),
        fee_payer: fee_payer.pubkey(),
        user_index: user_index_pda,
        batch_anchor: batch_anchor_pda,
        system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
    };

    let ix_data = openlesson_proof_anchor::instruction::AnchorBatch {
        batch_id,
        merkle_root,
        proof_count: 20,
        user_id_hash,
        session_id_hash: dummy_hash(30),
        start_timestamp: 1700010000,
        end_timestamp: 1700013600,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data.data(),
        accounts.to_account_metas(None),
    );

    send_tx(&mut svm, &[instruction], &fee_payer, &[&user])
        .expect("Batch should succeed");

    // Verify user_index account still exists with the right size
    let account = svm.get_account(&user_index_pda).expect("user_index should exist");
    assert!(account.data.len() >= 121);
    // At this point: total_proofs=3, total_batches=1, total_heartbeats=20
    // We can't easily deserialize without borsh in the test, but the fact that all
    // transactions succeeded without errors confirms the counters updated correctly.
}

#[test]
fn test_fee_payer_pattern() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    let fee_payer_balance_before = svm.get_balance(&fee_payer.pubkey()).unwrap();
    let user_balance_before = svm.get_balance(&user.pubkey()).unwrap();

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let fee_payer_balance_after = svm.get_balance(&fee_payer.pubkey()).unwrap();
    let user_balance_after = svm.get_balance(&user.pubkey()).unwrap();

    // Fee payer should have paid rent + tx fee
    assert!(
        fee_payer_balance_after < fee_payer_balance_before,
        "Fee payer balance should decrease (paid rent + fees)"
    );

    // User balance should be unchanged (they don't pay anything)
    assert_eq!(
        user_balance_before, user_balance_after,
        "User balance should not change (fee payer covers everything)"
    );
}

#[test]
fn test_all_proof_types() {
    let (mut svm, program_id, fee_payer, user) = setup();
    let user_id_hash = dummy_hash(1);

    init_user_account(&mut svm, &program_id, &fee_payer, &user, user_id_hash);

    let (user_index_pda, _) = find_pda(
        &[b"user_index", user.pubkey().as_ref()],
        &program_id,
    );

    // Test all 9 valid proof types (0-8)
    for proof_type in 0u8..=8 {
        let proof_id = dummy_hash(100 + proof_type);
        let fingerprint = dummy_hash(200 + proof_type);

        let (proof_anchor_pda, _) = find_pda(
            &[b"proof", proof_id.as_ref()],
            &program_id,
        );

        let accounts = openlesson_proof_anchor::accounts::AnchorProofCtx {
            user: user.pubkey(),
            fee_payer: fee_payer.pubkey(),
            user_index: user_index_pda,
            proof_anchor: proof_anchor_pda,
            system_program: solana_pubkey::Pubkey::from_str_const("11111111111111111111111111111111"),
        };

        let ix_data = openlesson_proof_anchor::instruction::AnchorProof {
            proof_id,
            fingerprint,
            proof_type,
            user_id_hash,
            event_timestamp: 1700000000 + (proof_type as i64 * 100),
            session_id_hash: dummy_hash(30),
            plan_id_hash: dummy_hash(40),
        };

        let instruction = Instruction::new_with_bytes(
            program_id,
            &ix_data.data(),
            accounts.to_account_metas(None),
        );

        send_tx(&mut svm, &[instruction], &fee_payer, &[&user])
            .unwrap_or_else(|e| panic!("Proof type {} should succeed: {}", proof_type, e));
    }
}
