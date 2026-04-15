import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

// ─── Proof Type Enum ─────────────────────────────────────────────────────────

/**
 * Maps proof type strings (from OpenLesson's agent-v2 API) to their on-chain u8 values.
 */
export const PROOF_TYPE_VALUES = {
  plan_created: 0,
  plan_adapted: 1,
  session_started: 2,
  session_paused: 3,
  session_resumed: 4,
  session_ended: 5,
  analysis_heartbeat: 6,
  assistant_query: 7,
  session_batch: 8,
} as const;

export type ProofType = keyof typeof PROOF_TYPE_VALUES;

/**
 * Reverse mapping: u8 value -> proof type string.
 */
export const PROOF_TYPE_NAMES: Record<number, ProofType> = Object.fromEntries(
  Object.entries(PROOF_TYPE_VALUES).map(([k, v]) => [v, k as ProofType]),
);

// ─── On-Chain Account Types ──────────────────────────────────────────────────

/**
 * Decoded UserProofIndex account data.
 */
export interface UserProofIndexAccount {
  userPubkey: PublicKey;
  userIdHash: number[];
  totalProofs: BN;
  totalBatches: BN;
  totalHeartbeats: BN;
  firstProofTimestamp: BN;
  lastProofTimestamp: BN;
  createdSlot: BN;
  bump: number;
}

/**
 * Decoded ProofAnchor account data.
 */
export interface ProofAnchorAccount {
  proofId: number[];
  fingerprint: number[];
  proofType: number;
  userPubkey: PublicKey;
  userIdHash: number[];
  eventTimestamp: BN;
  anchorSlot: BN;
  anchorTimestamp: BN;
  sessionIdHash: number[];
  planIdHash: number[];
  bump: number;
}

/**
 * Decoded BatchAnchor account data.
 */
export interface BatchAnchorAccount {
  batchId: number[];
  merkleRoot: number[];
  proofCount: number;
  userPubkey: PublicKey;
  userIdHash: number[];
  sessionIdHash: number[];
  startTimestamp: BN;
  endTimestamp: BN;
  anchorSlot: BN;
  anchorTimestamp: BN;
  bump: number;
}

// ─── SDK Input Types ─────────────────────────────────────────────────────────

/**
 * Input parameters for anchoring an individual proof.
 * All IDs/hashes can be passed as hex strings (with or without sha256: prefix),
 * UUIDs, or raw byte arrays.
 */
export interface AnchorProofParams {
  /** The proof's UUID (will be SHA-256 hashed to 32 bytes) */
  proofId: string;
  /** The proof fingerprint (sha256:hex or raw hex) */
  fingerprint: string;
  /** Proof type string or u8 value */
  proofType: ProofType | number;
  /** OpenLesson user ID (UUID, will be SHA-256 hashed) */
  userId: string;
  /** Unix timestamp of the original event (seconds) */
  eventTimestamp: number;
  /** Session ID (UUID, optional — zeroed if not provided) */
  sessionId?: string | null;
  /** Plan ID (UUID, optional — zeroed if not provided) */
  planId?: string | null;
}

/**
 * Input parameters for anchoring a batch of session proofs.
 */
export interface AnchorBatchParams {
  /** The batch's UUID (will be SHA-256 hashed to 32 bytes) */
  batchId: string;
  /** The Merkle root (sha256:hex or raw hex) */
  merkleRoot: string;
  /** Number of proofs in the batch */
  proofCount: number;
  /** OpenLesson user ID (UUID, will be SHA-256 hashed) */
  userId: string;
  /** Session ID (UUID, will be SHA-256 hashed) */
  sessionId: string;
  /** Unix timestamp of the first proof (seconds) */
  startTimestamp: number;
  /** Unix timestamp of the last proof (seconds) */
  endTimestamp: number;
}

// ─── PDA Seeds ───────────────────────────────────────────────────────────────

export const SEED_USER_INDEX = Buffer.from("user_index");
export const SEED_PROOF = Buffer.from("proof");
export const SEED_BATCH = Buffer.from("batch");
