// ─── OpenLesson Proof Anchor SDK ─────────────────────────────────────────────
//
// TypeScript client SDK for the OpenLesson Proof Anchor Solana program.
// Provides typed methods for anchoring learning proofs on-chain and reading
// them back for verification.
//
// Usage:
//
//   import { ProofAnchorClient } from "@openlesson/proof-anchor-sdk";
//   import { AnchorProvider } from "@coral-xyz/anchor";
//   import { Connection, Keypair } from "@solana/web3.js";
//
//   const connection = new Connection("http://localhost:8899");
//   const provider = AnchorProvider.local();
//   const feePayer = Keypair.fromSecretKey(...);
//
//   const client = new ProofAnchorClient(provider, feePayer);
//
// ─────────────────────────────────────────────────────────────────────────────

export { ProofAnchorClient } from "./client";

export {
  // Types
  type ProofType,
  type UserProofIndexAccount,
  type ProofAnchorAccount,
  type BatchAnchorAccount,
  type AnchorProofParams,
  type AnchorBatchParams,
  // Constants
  PROOF_TYPE_VALUES,
  PROOF_TYPE_NAMES,
  SEED_USER_INDEX,
  SEED_PROOF,
  SEED_BATCH,
} from "./types";

export {
  // Hash utilities
  sha256,
  uuidToBytes,
  fingerprintToBytes,
  hexToBytes,
  bytesToHex,
  bytesToFingerprint,
  zeroBytes,
  // Proof type helpers
  proofTypeToU8,
  u8ToProofType,
  // PDA derivation
  deriveUserIndexPDA,
  deriveProofPDA,
  deriveBatchPDA,
  // BN helpers
  timestampToBN,
} from "./utils";
