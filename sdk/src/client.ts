import {
  Program,
  AnchorProvider,
  type Idl,
  BN,
  type IdlAccounts,
} from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  type TransactionSignature,
  type Connection,
} from "@solana/web3.js";
import {
  type AnchorProofParams,
  type AnchorBatchParams,
  type ProofAnchorAccount,
  type BatchAnchorAccount,
  type UserProofIndexAccount,
} from "./types";
import {
  uuidToBytes,
  fingerprintToBytes,
  proofTypeToU8,
  zeroBytes,
  deriveUserIndexPDA,
  deriveProofPDA,
  deriveBatchPDA,
  timestampToBN,
  bytesToFingerprint,
  bytesToHex,
  u8ToProofType,
} from "./utils";

// Import the IDL — will be copied from target/idl at build time
import IDL_JSON from "../idl/openlesson_proof_anchor.json";

const IDL = IDL_JSON as Idl;

/**
 * High-level client for the OpenLesson Proof Anchor Solana program.
 *
 * Wraps the Anchor Program instance and provides typed methods for all
 * instructions and account fetches. Designed to be used by the OpenLesson
 * Next.js API (lib/agent-v2/solana.ts) or any other TypeScript consumer.
 *
 * @example
 * ```ts
 * const client = new ProofAnchorClient(provider, feePayer);
 *
 * // Initialize user (once per user)
 * await client.initializeUserAccount(userKeypair, "user-uuid-123");
 *
 * // Anchor a proof
 * const txSig = await client.anchorProof(userKeypair, {
 *   proofId: "proof-uuid-456",
 *   fingerprint: "sha256:abcdef...",
 *   proofType: "session_started",
 *   userId: "user-uuid-123",
 *   eventTimestamp: Math.floor(Date.now() / 1000),
 *   sessionId: "session-uuid-789",
 * });
 *
 * // Read back the on-chain data
 * const proofData = await client.fetchProofAnchor("proof-uuid-456");
 * ```
 */
export class ProofAnchorClient {
  public readonly program: Program;
  public readonly programId: PublicKey;
  private readonly feePayer: Keypair;

  /**
   * @param provider - Anchor provider (connection + wallet)
   * @param feePayer - The fee payer keypair (OpenLesson's wallet that covers all tx fees)
   * @param programId - Optional override for the program ID (defaults to IDL address)
   */
  constructor(
    provider: AnchorProvider,
    feePayer: Keypair,
    programId?: PublicKey,
  ) {
    this.programId = programId || new PublicKey(IDL_JSON.address);
    this.program = new Program(IDL, provider);
    this.feePayer = feePayer;
  }

  /**
   * Get the underlying Solana connection.
   */
  get connection(): Connection {
    return this.program.provider.connection;
  }

  // ─── Instructions ────────────────────────────────────────────────────────

  /**
   * Initialize a user's proof index account. Must be called once per user
   * before they can anchor any proofs.
   *
   * @param userKeypair - The user's custodial wallet keypair (signs to prove ownership)
   * @param userId - The OpenLesson user ID (UUID string)
   * @returns Transaction signature
   */
  async initializeUserAccount(
    userKeypair: Keypair,
    userId: string,
  ): Promise<TransactionSignature> {
    const userIdHash = uuidToBytes(userId);
    const [userIndexPDA] = deriveUserIndexPDA(
      userKeypair.publicKey,
      this.programId,
    );

    const tx = await (this.program.methods as any)
      .initializeUserAccount(userIdHash)
      .accounts({
        user: userKeypair.publicKey,
        feePayer: this.feePayer.publicKey,
        userIndex: userIndexPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([userKeypair, this.feePayer])
      .rpc();

    return tx;
  }

  /**
   * Check if a user's proof index account already exists.
   */
  async isUserInitialized(userPubkey: PublicKey): Promise<boolean> {
    const [userIndexPDA] = deriveUserIndexPDA(userPubkey, this.programId);
    const account = await this.connection.getAccountInfo(userIndexPDA);
    return account !== null;
  }

  /**
   * Initialize the user account if it doesn't exist yet. No-op if already initialized.
   *
   * @returns Transaction signature if initialized, null if already exists
   */
  async ensureUserInitialized(
    userKeypair: Keypair,
    userId: string,
  ): Promise<TransactionSignature | null> {
    const initialized = await this.isUserInitialized(userKeypair.publicKey);
    if (initialized) return null;
    return this.initializeUserAccount(userKeypair, userId);
  }

  /**
   * Anchor an individual proof on-chain.
   *
   * @param userKeypair - The user's custodial wallet keypair
   * @param params - Proof parameters
   * @returns Transaction signature
   */
  async anchorProof(
    userKeypair: Keypair,
    params: AnchorProofParams,
  ): Promise<TransactionSignature> {
    const proofIdBytes = uuidToBytes(params.proofId);
    const fingerprintBytes = fingerprintToBytes(params.fingerprint);
    const proofType = proofTypeToU8(params.proofType);
    const userIdHash = uuidToBytes(params.userId);
    const sessionIdHash = params.sessionId
      ? uuidToBytes(params.sessionId)
      : zeroBytes();
    const planIdHash = params.planId ? uuidToBytes(params.planId) : zeroBytes();

    const [userIndexPDA] = deriveUserIndexPDA(
      userKeypair.publicKey,
      this.programId,
    );
    const [proofAnchorPDA] = deriveProofPDA(proofIdBytes, this.programId);

    const tx = await (this.program.methods as any)
      .anchorProof(
        proofIdBytes,
        fingerprintBytes,
        proofType,
        userIdHash,
        timestampToBN(params.eventTimestamp),
        sessionIdHash,
        planIdHash,
      )
      .accounts({
        user: userKeypair.publicKey,
        feePayer: this.feePayer.publicKey,
        userIndex: userIndexPDA,
        proofAnchor: proofAnchorPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([userKeypair, this.feePayer])
      .rpc();

    return tx;
  }

  /**
   * Anchor a batch of session proofs (Merkle root) on-chain.
   *
   * @param userKeypair - The user's custodial wallet keypair
   * @param params - Batch parameters
   * @returns Transaction signature
   */
  async anchorBatch(
    userKeypair: Keypair,
    params: AnchorBatchParams,
  ): Promise<TransactionSignature> {
    const batchIdBytes = uuidToBytes(params.batchId);
    const merkleRootBytes = fingerprintToBytes(params.merkleRoot);
    const userIdHash = uuidToBytes(params.userId);
    const sessionIdHash = uuidToBytes(params.sessionId);

    const [userIndexPDA] = deriveUserIndexPDA(
      userKeypair.publicKey,
      this.programId,
    );
    const [batchAnchorPDA] = deriveBatchPDA(batchIdBytes, this.programId);

    const tx = await (this.program.methods as any)
      .anchorBatch(
        batchIdBytes,
        merkleRootBytes,
        params.proofCount,
        userIdHash,
        sessionIdHash,
        timestampToBN(params.startTimestamp),
        timestampToBN(params.endTimestamp),
      )
      .accounts({
        user: userKeypair.publicKey,
        feePayer: this.feePayer.publicKey,
        userIndex: userIndexPDA,
        batchAnchor: batchAnchorPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([userKeypair, this.feePayer])
      .rpc();

    return tx;
  }

  // ─── Account Fetching ────────────────────────────────────────────────────

  /**
   * Fetch a user's proof index account data.
   *
   * @param userPubkey - The user's Solana public key
   * @returns Decoded UserProofIndex data, or null if not found
   */
  async fetchUserIndex(
    userPubkey: PublicKey,
  ): Promise<UserProofIndexAccount | null> {
    const [pda] = deriveUserIndexPDA(userPubkey, this.programId);
    try {
      const account = await (this.program.account as any).userProofIndex.fetch(
        pda,
      );
      return account as UserProofIndexAccount;
    } catch {
      return null;
    }
  }

  /**
   * Fetch a proof anchor account by proof ID.
   *
   * @param proofId - UUID string or 32-byte array
   * @returns Decoded ProofAnchor data, or null if not found
   */
  async fetchProofAnchor(
    proofId: string | number[],
  ): Promise<ProofAnchorAccount | null> {
    const [pda] = deriveProofPDA(proofId, this.programId);
    try {
      const account = await (this.program.account as any).proofAnchor.fetch(
        pda,
      );
      return account as ProofAnchorAccount;
    } catch {
      return null;
    }
  }

  /**
   * Fetch a batch anchor account by batch ID.
   *
   * @param batchId - UUID string or 32-byte array
   * @returns Decoded BatchAnchor data, or null if not found
   */
  async fetchBatchAnchor(
    batchId: string | number[],
  ): Promise<BatchAnchorAccount | null> {
    const [pda] = deriveBatchPDA(batchId, this.programId);
    try {
      const account = await (this.program.account as any).batchAnchor.fetch(
        pda,
      );
      return account as BatchAnchorAccount;
    } catch {
      return null;
    }
  }

  // ─── Verification Helpers ────────────────────────────────────────────────

  /**
   * Verify that a proof exists on-chain and its fingerprint matches.
   *
   * @param proofId - UUID string
   * @param expectedFingerprint - Expected fingerprint (sha256:hex)
   * @returns Verification result
   */
  async verifyProofOnChain(
    proofId: string,
    expectedFingerprint: string,
  ): Promise<{
    exists: boolean;
    fingerprintMatch: boolean;
    onChainFingerprint: string | null;
    anchorSlot: number | null;
    anchorTimestamp: number | null;
  }> {
    const account = await this.fetchProofAnchor(proofId);
    if (!account) {
      return {
        exists: false,
        fingerprintMatch: false,
        onChainFingerprint: null,
        anchorSlot: null,
        anchorTimestamp: null,
      };
    }

    const onChainFingerprint = bytesToFingerprint(account.fingerprint);
    return {
      exists: true,
      fingerprintMatch: onChainFingerprint === expectedFingerprint,
      onChainFingerprint,
      anchorSlot: account.anchorSlot.toNumber(),
      anchorTimestamp: account.anchorTimestamp.toNumber(),
    };
  }

  /**
   * Verify that a batch exists on-chain and its Merkle root matches.
   *
   * @param batchId - UUID string
   * @param expectedMerkleRoot - Expected Merkle root (sha256:hex)
   * @returns Verification result
   */
  async verifyBatchOnChain(
    batchId: string,
    expectedMerkleRoot: string,
  ): Promise<{
    exists: boolean;
    merkleRootMatch: boolean;
    onChainMerkleRoot: string | null;
    proofCount: number | null;
    anchorSlot: number | null;
    anchorTimestamp: number | null;
  }> {
    const account = await this.fetchBatchAnchor(batchId);
    if (!account) {
      return {
        exists: false,
        merkleRootMatch: false,
        onChainMerkleRoot: null,
        proofCount: null,
        anchorSlot: null,
        anchorTimestamp: null,
      };
    }

    const onChainMerkleRoot = bytesToFingerprint(account.merkleRoot);
    return {
      exists: true,
      merkleRootMatch: onChainMerkleRoot === expectedMerkleRoot,
      onChainMerkleRoot,
      proofCount: account.proofCount,
      anchorSlot: account.anchorSlot.toNumber(),
      anchorTimestamp: account.anchorTimestamp.toNumber(),
    };
  }

  // ─── PDA Address Helpers ─────────────────────────────────────────────────

  /**
   * Get the PDA address for a user's proof index.
   */
  getUserIndexAddress(userPubkey: PublicKey): PublicKey {
    const [pda] = deriveUserIndexPDA(userPubkey, this.programId);
    return pda;
  }

  /**
   * Get the PDA address for a proof anchor.
   */
  getProofAnchorAddress(proofId: string | number[]): PublicKey {
    const [pda] = deriveProofPDA(proofId, this.programId);
    return pda;
  }

  /**
   * Get the PDA address for a batch anchor.
   */
  getBatchAnchorAddress(batchId: string | number[]): PublicKey {
    const [pda] = deriveBatchPDA(batchId, this.programId);
    return pda;
  }
}
