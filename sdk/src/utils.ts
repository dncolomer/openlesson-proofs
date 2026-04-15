import { createHash } from "crypto";
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  SEED_USER_INDEX,
  SEED_PROOF,
  SEED_BATCH,
  PROOF_TYPE_VALUES,
  PROOF_TYPE_NAMES,
  type ProofType,
} from "./types";

// ─── Hash / Byte Conversion Utilities ────────────────────────────────────────

/**
 * SHA-256 hash a string and return the raw 32-byte Buffer.
 */
export function sha256(data: string): Buffer {
  return createHash("sha256").update(data).digest();
}

/**
 * Convert a UUID string to a 32-byte array (SHA-256 hash of the UUID).
 * This is the canonical way to convert OpenLesson UUIDs to on-chain [u8; 32].
 */
export function uuidToBytes(uuid: string): number[] {
  return Array.from(sha256(uuid));
}

/**
 * Convert a fingerprint string to a 32-byte array.
 * Accepts formats: "sha256:abcdef..." or raw hex "abcdef..."
 */
export function fingerprintToBytes(fingerprint: string): number[] {
  const hex = fingerprint.startsWith("sha256:")
    ? fingerprint.slice(7)
    : fingerprint;
  if (hex.length !== 64) {
    throw new Error(
      `Invalid fingerprint hex length: expected 64, got ${hex.length}`,
    );
  }
  return Array.from(Buffer.from(hex, "hex"));
}

/**
 * Convert a hex string (with optional sha256: prefix) to a 32-byte array.
 */
export function hexToBytes(hex: string): number[] {
  const raw = hex.startsWith("sha256:") ? hex.slice(7) : hex;
  return Array.from(Buffer.from(raw, "hex"));
}

/**
 * Convert a 32-byte array to a hex string.
 */
export function bytesToHex(bytes: number[]): string {
  return Buffer.from(bytes).toString("hex");
}

/**
 * Convert a 32-byte array to a fingerprint string (with sha256: prefix).
 */
export function bytesToFingerprint(bytes: number[]): string {
  return `sha256:${bytesToHex(bytes)}`;
}

/**
 * Create a zeroed 32-byte array (used for optional fields like session_id when absent).
 */
export function zeroBytes(): number[] {
  return new Array(32).fill(0);
}

// ─── Proof Type Conversions ──────────────────────────────────────────────────

/**
 * Convert a ProofType string to its u8 on-chain value.
 */
export function proofTypeToU8(proofType: ProofType | number): number {
  if (typeof proofType === "number") {
    if (proofType < 0 || proofType > 8) {
      throw new Error(`Invalid proof type value: ${proofType}. Must be 0-8.`);
    }
    return proofType;
  }
  const value = PROOF_TYPE_VALUES[proofType];
  if (value === undefined) {
    throw new Error(`Unknown proof type: ${proofType}`);
  }
  return value;
}

/**
 * Convert a u8 on-chain proof type value to its string name.
 */
export function u8ToProofType(value: number): ProofType {
  const name = PROOF_TYPE_NAMES[value];
  if (!name) {
    throw new Error(`Unknown proof type value: ${value}. Must be 0-8.`);
  }
  return name;
}

// ─── PDA Derivation ──────────────────────────────────────────────────────────

/**
 * Derive the UserProofIndex PDA address.
 * Seeds: ["user_index", user_pubkey]
 */
export function deriveUserIndexPDA(
  userPubkey: PublicKey,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_USER_INDEX, userPubkey.toBuffer()],
    programId,
  );
}

/**
 * Derive the ProofAnchor PDA address.
 * Seeds: ["proof", proof_id_bytes]
 *
 * @param proofId - UUID string (will be SHA-256 hashed) or 32-byte array
 */
export function deriveProofPDA(
  proofId: string | number[],
  programId: PublicKey,
): [PublicKey, number] {
  const bytes = typeof proofId === "string" ? uuidToBytes(proofId) : proofId;
  return PublicKey.findProgramAddressSync(
    [SEED_PROOF, Buffer.from(bytes)],
    programId,
  );
}

/**
 * Derive the BatchAnchor PDA address.
 * Seeds: ["batch", batch_id_bytes]
 *
 * @param batchId - UUID string (will be SHA-256 hashed) or 32-byte array
 */
export function deriveBatchPDA(
  batchId: string | number[],
  programId: PublicKey,
): [PublicKey, number] {
  const bytes = typeof batchId === "string" ? uuidToBytes(batchId) : batchId;
  return PublicKey.findProgramAddressSync(
    [SEED_BATCH, Buffer.from(bytes)],
    programId,
  );
}

// ─── BN Helpers ──────────────────────────────────────────────────────────────

/**
 * Convert a Unix timestamp (seconds) to a BN for instruction args.
 */
export function timestampToBN(timestamp: number): BN {
  return new BN(timestamp);
}
