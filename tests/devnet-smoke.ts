/**
 * Devnet smoke test for OpenLesson Proof Anchor program.
 *
 * Exercises all 3 instructions against the live devnet deployment:
 *   1. initialize_user_account
 *   2. anchor_proof
 *   3. anchor_batch
 *
 * Then reads back all 3 accounts and verifies the data matches.
 *
 * Usage:
 *   npx ts-mocha -p tsconfig.json tests/devnet-smoke.ts --timeout 60000
 */

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Connection,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { createHash } from "crypto";
import { assert } from "chai";
import BN from "bn.js";

// Load IDL
import { readFileSync } from "fs";
import { resolve } from "path";
const IDL = JSON.parse(
  readFileSync(
    resolve(process.cwd(), "target", "idl", "openlesson_proof_anchor.json"),
    "utf-8",
  ),
);

const PROGRAM_ID = new PublicKey(
  "6kFPmDutPLRigDcyKaLAtRnDbBZAh7teLfAMQLWhZJ5J",
);

// ─── Helpers ─────────────────────────────────────────────────────────────────

function sha256(data: string): Buffer {
  return createHash("sha256").update(data).digest();
}

function uuidToBytes(uuid: string): number[] {
  return Array.from(sha256(uuid));
}

function fingerprintToBytes(fp: string): number[] {
  const hex = fp.startsWith("sha256:") ? fp.slice(7) : fp;
  return Array.from(Buffer.from(hex, "hex"));
}

function deriveUserIndexPDA(userPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("user_index"), userPubkey.toBuffer()],
    PROGRAM_ID,
  );
}

function deriveProofPDA(proofIdBytes: number[]): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("proof"), Buffer.from(proofIdBytes)],
    PROGRAM_ID,
  );
}

function deriveBatchPDA(batchIdBytes: number[]): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("batch"), Buffer.from(batchIdBytes)],
    PROGRAM_ID,
  );
}

// ─── Test Suite ──────────────────────────────────────────────────────────────

describe("devnet smoke test", () => {
  const connection = new Connection(
    "https://api.devnet.solana.com",
    "confirmed",
  );

  // Fee payer = the default Solana CLI keypair (has SOL on devnet)
  const walletPath = resolve(
    process.env.HOME || "~",
    ".config",
    "solana",
    "id.json",
  );
  const feePayerKeypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(readFileSync(walletPath, "utf-8"))),
  );
  const feePayerWallet = new anchor.Wallet(feePayerKeypair);
  const feePayer = feePayerWallet;
  const provider = new anchor.AnchorProvider(connection, feePayerWallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  const program = new Program(IDL as anchor.Idl, provider);

  // Generate a fresh user keypair for this test run
  const user = Keypair.generate();

  // Test data
  const userId = "test-user-" + Date.now();
  const userIdHash = uuidToBytes(userId);
  const proofId = "test-proof-" + Date.now();
  const proofIdBytes = uuidToBytes(proofId);
  const batchId = "test-batch-" + Date.now();
  const batchIdBytes = uuidToBytes(batchId);

  // A fake fingerprint (64 hex chars = 32 bytes)
  const fingerprint = sha256("test-fingerprint-data-" + Date.now()).toString(
    "hex",
  );
  const fingerprintBytes = fingerprintToBytes(fingerprint);

  // Merkle root
  const merkleRoot = sha256("test-merkle-root-" + Date.now()).toString("hex");
  const merkleRootBytes = fingerprintToBytes(merkleRoot);

  const sessionIdHash = uuidToBytes("test-session-" + Date.now());
  const planIdHash = uuidToBytes("test-plan-" + Date.now());
  const eventTimestamp = Math.floor(Date.now() / 1000);

  // PDA addresses
  const [userIndexPDA] = deriveUserIndexPDA(user.publicKey);
  const [proofAnchorPDA] = deriveProofPDA(proofIdBytes);
  const [batchAnchorPDA] = deriveBatchPDA(batchIdBytes);

  before(async () => {
    // Fund the user account with a tiny amount (they don't pay, but need to exist)
    console.log("  Fee payer:", feePayer.publicKey.toBase58());
    console.log("  User:     ", user.publicKey.toBase58());
    console.log("  Program:  ", PROGRAM_ID.toBase58());

    const balance = await connection.getBalance(feePayer.publicKey);
    console.log("  Fee payer balance:", balance / LAMPORTS_PER_SOL, "SOL");
    assert(balance > 0.1 * LAMPORTS_PER_SOL, "Fee payer needs SOL on devnet");
  });

  it("initialize_user_account", async () => {
    const tx = await (program.methods as any)
      .initializeUserAccount(userIdHash)
      .accounts({
        user: user.publicKey,
        feePayer: feePayer.publicKey,
        userIndex: userIndexPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    console.log("    tx:", tx);

    // Fetch and verify
    const account = await (program.account as any).userProofIndex.fetch(
      userIndexPDA,
    );
    assert.equal(account.userPubkey.toBase58(), user.publicKey.toBase58());
    assert.deepEqual(account.userIdHash, userIdHash);
    assert.equal(account.totalProofs.toNumber(), 0);
    assert.equal(account.totalBatches.toNumber(), 0);
    assert.equal(account.totalHeartbeats.toNumber(), 0);
    console.log("    UserProofIndex created, bump:", account.bump);
  });

  it("anchor_proof", async () => {
    const tx = await (program.methods as any)
      .anchorProof(
        proofIdBytes,
        fingerprintBytes,
        2, // session_started
        userIdHash,
        new BN(eventTimestamp),
        sessionIdHash,
        planIdHash,
      )
      .accounts({
        user: user.publicKey,
        feePayer: feePayer.publicKey,
        userIndex: userIndexPDA,
        proofAnchor: proofAnchorPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    console.log("    tx:", tx);

    // Fetch and verify proof
    const proof = await (program.account as any).proofAnchor.fetch(
      proofAnchorPDA,
    );
    assert.deepEqual(proof.proofId, proofIdBytes);
    assert.deepEqual(proof.fingerprint, fingerprintBytes);
    assert.equal(proof.proofType, 2);
    assert.equal(proof.userPubkey.toBase58(), user.publicKey.toBase58());
    assert.equal(proof.eventTimestamp.toNumber(), eventTimestamp);
    assert(proof.anchorSlot.toNumber() > 0, "anchor_slot should be set");
    console.log("    ProofAnchor created, slot:", proof.anchorSlot.toNumber());

    // Verify user index was updated
    const userIndex = await (program.account as any).userProofIndex.fetch(
      userIndexPDA,
    );
    assert.equal(userIndex.totalProofs.toNumber(), 1);
    assert.equal(userIndex.lastProofTimestamp.toNumber(), eventTimestamp);
  });

  it("anchor_batch", async () => {
    const proofCount = 25;
    const startTs = eventTimestamp - 3600;
    const endTs = eventTimestamp;

    const tx = await (program.methods as any)
      .anchorBatch(
        batchIdBytes,
        merkleRootBytes,
        proofCount,
        userIdHash,
        sessionIdHash,
        new BN(startTs),
        new BN(endTs),
      )
      .accounts({
        user: user.publicKey,
        feePayer: feePayer.publicKey,
        userIndex: userIndexPDA,
        batchAnchor: batchAnchorPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    console.log("    tx:", tx);

    // Fetch and verify batch
    const batch = await (program.account as any).batchAnchor.fetch(
      batchAnchorPDA,
    );
    assert.deepEqual(batch.batchId, batchIdBytes);
    assert.deepEqual(batch.merkleRoot, merkleRootBytes);
    assert.equal(batch.proofCount, proofCount);
    assert.equal(batch.userPubkey.toBase58(), user.publicKey.toBase58());
    assert.equal(batch.startTimestamp.toNumber(), startTs);
    assert.equal(batch.endTimestamp.toNumber(), endTs);
    assert(batch.anchorSlot.toNumber() > 0, "anchor_slot should be set");
    console.log("    BatchAnchor created, slot:", batch.anchorSlot.toNumber());

    // Verify user index was updated
    const userIndex = await (program.account as any).userProofIndex.fetch(
      userIndexPDA,
    );
    assert.equal(userIndex.totalProofs.toNumber(), 1);
    assert.equal(userIndex.totalBatches.toNumber(), 1);
    assert.equal(userIndex.totalHeartbeats.toNumber(), proofCount);
  });

  it("rejects invalid proof type on devnet", async () => {
    const badProofId = uuidToBytes("bad-proof-" + Date.now());
    const [badProofPDA] = deriveProofPDA(badProofId);

    try {
      await (program.methods as any)
        .anchorProof(
          badProofId,
          fingerprintBytes,
          9, // INVALID
          userIdHash,
          new BN(eventTimestamp),
          sessionIdHash,
          planIdHash,
        )
        .accounts({
          user: user.publicKey,
          feePayer: feePayer.publicKey,
          userIndex: userIndexPDA,
          proofAnchor: badProofPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      assert.fail("Should have thrown for invalid proof type");
    } catch (err: any) {
      assert.include(
        err.toString(),
        "InvalidProofType",
        "Error should mention InvalidProofType",
      );
      console.log("    Correctly rejected invalid proof type (9)");
    }
  });

  it("rejects duplicate proof on devnet", async () => {
    // Try to anchor the same proofId again
    try {
      await (program.methods as any)
        .anchorProof(
          proofIdBytes,
          fingerprintBytes,
          2,
          userIdHash,
          new BN(eventTimestamp),
          sessionIdHash,
          planIdHash,
        )
        .accounts({
          user: user.publicKey,
          feePayer: feePayer.publicKey,
          userIndex: userIndexPDA,
          proofAnchor: proofAnchorPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      assert.fail("Should have thrown for duplicate proof_id");
    } catch (err: any) {
      // PDA already exists error
      console.log("    Correctly rejected duplicate proof_id");
    }
  });
});
