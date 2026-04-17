# OpenLesson Proof Anchor

Solana program for anchoring cryptographic proofs of learning activities on-chain. Part of the [OpenLesson](https://openlesson.academy) platform — enables trustless, permanent verification that educational sessions, plans, and analyses actually occurred.

Every significant learning event (session started, analysis heartbeat, plan created, etc.) produces a SHA-256 fingerprint off-chain. This program stores those fingerprints on Solana, making them publicly verifiable and tamper-proof.

## How It Works

```
OpenLesson API                          Solana
─────────────                          ──────
User starts session
  → SHA-256 fingerprint generated
  → Stored in Supabase (agent_proofs)
  → anchor_proof() ──────────────────→ ProofAnchor PDA created
                                        (fingerprint stored on-chain)

Analysis heartbeats (every ~10s)
  → Individual proofs stored locally
  → NOT anchored individually

Session ends
  → All heartbeats aggregated
  → Merkle tree built
  → anchor_batch() ──────────────────→ BatchAnchor PDA created
                                        (Merkle root stored on-chain)

Anyone can verify
  → Fetch on-chain fingerprint
  → Compare to off-chain data
  → Verify Merkle proof for heartbeats
```

### Anchoring Strategy

Not everything goes on-chain individually. The program uses a batching strategy to minimize costs:

| Event                                | On-Chain Behavior                           |
| ------------------------------------ | ------------------------------------------- |
| Plan created/adapted                 | Immediately anchored as individual proof    |
| Session started/paused/resumed/ended | Immediately anchored as individual proof    |
| Analysis heartbeat                   | Stored off-chain, batched at session end    |
| Assistant query                      | Stored off-chain, batched at session end    |
| Session batch                        | Merkle root of all heartbeats anchored once |

A single tutoring session with 100 heartbeats results in only **2-3 on-chain transactions** (session_started + session_ended + batch), not 100.

## Architecture

### On-Chain Accounts (PDAs)

**UserProofIndex** — One per user. Tracks aggregate statistics.

```
Seeds: ["user_index", user_pubkey]
Size:  121 bytes

Fields:
  user_pubkey          Pubkey    User's Solana public key
  user_id_hash         [u8; 32] SHA-256 of OpenLesson user UUID
  total_proofs         u64      Total individual proofs anchored
  total_batches        u64      Total batch proofs anchored
  total_heartbeats     u64      Sum of all batch proof_counts
  first_proof_timestamp i64     Unix timestamp of first proof
  last_proof_timestamp  i64     Unix timestamp of most recent proof
  created_slot         u64      Solana slot when account was created
  bump                 u8       PDA bump seed
```

**ProofAnchor** — One per individual proof. Stores the fingerprint.

```
Seeds: ["proof", proof_id]
Size:  226 bytes

Fields:
  proof_id             [u8; 32] SHA-256 of proof UUID
  fingerprint          [u8; 32] SHA-256 fingerprint of event data
  proof_type           u8       Enum value (0-8, see below)
  user_pubkey          Pubkey   User's Solana public key
  user_id_hash         [u8; 32] SHA-256 of OpenLesson user UUID
  event_timestamp      i64      When the original event occurred
  anchor_slot          u64      Solana slot when anchored
  anchor_timestamp     i64      Unix timestamp when anchored
  session_id_hash      [u8; 32] Related session (zeroed if none)
  plan_id_hash         [u8; 32] Related plan (zeroed if none)
  bump                 u8       PDA bump seed
```

**BatchAnchor** — One per session batch. Stores the Merkle root.

```
Seeds: ["batch", batch_id]
Size:  205 bytes

Fields:
  batch_id             [u8; 32] SHA-256 of batch UUID
  merkle_root          [u8; 32] Merkle root of all proof fingerprints
  proof_count          u32      Number of proofs in the batch
  user_pubkey          Pubkey   User's Solana public key
  user_id_hash         [u8; 32] SHA-256 of OpenLesson user UUID
  session_id_hash      [u8; 32] Session this batch belongs to
  start_timestamp      i64      First proof timestamp
  end_timestamp        i64      Last proof timestamp
  anchor_slot          u64      Solana slot when anchored
  anchor_timestamp     i64      Unix timestamp when anchored
  bump                 u8       PDA bump seed
```

### Instructions

**`initialize_user_account(user_id_hash)`**

Creates the `UserProofIndex` PDA for a new user. Called once per user before any proofs can be anchored. The fee payer (OpenLesson) covers rent; the user signs to prove wallet ownership.

**`anchor_proof(proof_id, fingerprint, proof_type, user_id_hash, event_timestamp, session_id_hash, plan_id_hash)`**

Stores an individual proof fingerprint on-chain. Creates a `ProofAnchor` PDA and increments the user's `total_proofs` counter. Validates that `proof_type` is 0-8. Emits a `ProofAnchored` event.

**`anchor_batch(batch_id, merkle_root, proof_count, user_id_hash, session_id_hash, start_timestamp, end_timestamp)`**

Stores a session's Merkle root on-chain. Creates a `BatchAnchor` PDA and increments the user's `total_batches` and `total_heartbeats` counters. Validates that `proof_count` is 1-1000 and `end_timestamp >= start_timestamp`. Emits a `BatchAnchored` event.

### Proof Types

| Value | Name                 | Description                               |
| ----- | -------------------- | ----------------------------------------- |
| 0     | `plan_created`       | Learning plan created                     |
| 1     | `plan_adapted`       | Learning plan modified by AI              |
| 2     | `session_started`    | Tutoring session started                  |
| 3     | `session_paused`     | Session paused                            |
| 4     | `session_resumed`    | Session resumed                           |
| 5     | `session_ended`      | Session ended                             |
| 6     | `analysis_heartbeat` | Analysis chunk processed (~10s intervals) |
| 7     | `assistant_query`    | Teaching assistant queried                |
| 8     | `session_batch`      | Merkle root of session heartbeats         |

### Fee Payer Pattern

Every instruction takes two signers:

- **`user`** — The user's custodial wallet. Signs to prove ownership. Never pays.
- **`fee_payer`** — OpenLesson's wallet. Pays all rent and transaction fees.

This means users never need SOL. OpenLesson manages custodial wallets (AES-256-GCM encrypted private keys stored in Supabase) and a fee payer wallet that covers all costs.

### Events

**`ProofAnchored`** — Emitted when an individual proof is anchored.

```
proof_id, fingerprint, proof_type, user_pubkey, event_timestamp, anchor_slot
```

**`BatchAnchored`** — Emitted when a session batch is anchored.

```
batch_id, merkle_root, proof_count, user_pubkey, session_id_hash, anchor_slot
```

### Error Codes

| Code | Name                | Condition                         |
| ---- | ------------------- | --------------------------------- |
| 6000 | `InvalidProofType`  | `proof_type > 8`                  |
| 6001 | `EmptyBatch`        | `proof_count == 0`                |
| 6002 | `BatchTooLarge`     | `proof_count > 1000`              |
| 6003 | `InvalidTimestamps` | `end_timestamp < start_timestamp` |

Duplicate prevention is handled by PDA uniqueness — attempting to anchor the same `proof_id` or `batch_id` twice fails because the PDA already exists.

## Project Structure

```
openlesson-proofs/
├── programs/
│   └── openlesson-proof-anchor/
│       ├── src/
│       │   ├── lib.rs                         # Program entrypoint
│       │   ├── state.rs                       # Account structs (3 PDAs)
│       │   ├── constants.rs                   # PDA seeds, limits
│       │   ├── error.rs                       # Custom error codes
│       │   ├── events.rs                      # On-chain events
│       │   └── instructions/
│       │       ├── initialize_user_account.rs # Create user index
│       │       ├── anchor_proof.rs            # Anchor individual proof
│       │       └── anchor_batch.rs            # Anchor session batch
│       ├── tests/
│       │   └── test_proof_anchor.rs           # 11 integration tests
│       └── Cargo.toml
├── sdk/
│   ├── src/
│   │   ├── index.ts                           # Public exports
│   │   ├── client.ts                          # ProofAnchorClient class
│   │   ├── types.ts                           # TypeScript types
│   │   └── utils.ts                           # Hash, PDA, conversion helpers
│   ├── idl/
│   │   └── openlesson_proof_anchor.json       # Generated IDL
│   ├── package.json
│   └── tsconfig.json
├── Anchor.toml
├── Cargo.toml
├── package.json
└── rust-toolchain.toml
```

## Prerequisites

- **Rust** 1.89+ (via [rustup](https://rustup.rs))
- **Solana CLI** 3.x (via [Agave](https://docs.anza.xyz/cli/install))
- **Anchor CLI** 1.0 (via [avm](https://www.anchor-lang.com/docs/installation))
- **Node.js** 18+ (for SDK)

## Quick Start

### Build

```bash
anchor build
```

This compiles the program to `target/deploy/openlesson_proof_anchor.so` and generates the IDL at `target/idl/openlesson_proof_anchor.json`.

### Test

```bash
cargo test
```

Runs 11 integration tests using [LiteSVM](https://github.com/LiteSVM/litesvm) (in-process Solana VM, no validator needed):

```
test test_initialize_user_account .............. ok
test test_anchor_proof ......................... ok
test test_anchor_proof_invalid_type ............ ok
test test_anchor_proof_duplicate_prevention ..... ok
test test_anchor_batch ......................... ok
test test_anchor_batch_empty ................... ok
test test_anchor_batch_too_large ............... ok
test test_anchor_batch_invalid_timestamps ...... ok
test test_multiple_proofs_user_index_tracking .. ok
test test_fee_payer_pattern .................... ok
test test_all_proof_types ...................... ok

test result: ok. 11 passed; 0 failed
```

### SDK

```bash
cd sdk
npm install
npm run build    # Outputs to sdk/dist/
```

## SDK Usage

The TypeScript SDK wraps the Anchor program with a typed client for use in server-side code (e.g., the OpenLesson Next.js API).

### Setup

```typescript
import { ProofAnchorClient } from "@openlesson/proof-anchor-sdk";
import { AnchorProvider } from "@coral-xyz/anchor";
import { Connection, Keypair } from "@solana/web3.js";

const connection = new Connection("http://localhost:8899");
const wallet = /* AnchorProvider wallet */;
const provider = new AnchorProvider(connection, wallet, {});
const feePayer = Keypair.fromSecretKey(/* OpenLesson fee payer key */);

const client = new ProofAnchorClient(provider, feePayer);
```

### Initialize a User

```typescript
const userKeypair = Keypair.generate(); // or load custodial wallet

// Only needs to be called once per user
await client.initializeUserAccount(userKeypair, "user-uuid-123");

// Or use the idempotent version:
await client.ensureUserInitialized(userKeypair, "user-uuid-123");
```

### Anchor a Proof

```typescript
const txSignature = await client.anchorProof(userKeypair, {
  proofId: "proof-uuid-456",
  fingerprint: "sha256:a1b2c3d4e5f6...",
  proofType: "session_started", // or numeric: 2
  userId: "user-uuid-123",
  eventTimestamp: Math.floor(Date.now() / 1000),
  sessionId: "session-uuid-789", // optional
  planId: "plan-uuid-012", // optional
});
```

### Anchor a Batch

```typescript
const txSignature = await client.anchorBatch(userKeypair, {
  batchId: "batch-uuid-345",
  merkleRoot: "sha256:f1e2d3c4b5a6...",
  proofCount: 47,
  userId: "user-uuid-123",
  sessionId: "session-uuid-789",
  startTimestamp: 1700000000,
  endTimestamp: 1700003600,
});
```

### Verify On-Chain

```typescript
// Verify individual proof
const result = await client.verifyProofOnChain(
  "proof-uuid-456",
  "sha256:a1b2c3d4e5f6...",
);
// { exists: true, fingerprintMatch: true, anchorSlot: 12345, ... }

// Verify batch
const batchResult = await client.verifyBatchOnChain(
  "batch-uuid-345",
  "sha256:f1e2d3c4b5a6...",
);
// { exists: true, merkleRootMatch: true, proofCount: 47, ... }
```

### Read Account Data

```typescript
// User stats
const userIndex = await client.fetchUserIndex(userKeypair.publicKey);
// { totalProofs: 15, totalBatches: 3, totalHeartbeats: 142, ... }

// Individual proof
const proof = await client.fetchProofAnchor("proof-uuid-456");

// Session batch
const batch = await client.fetchBatchAnchor("batch-uuid-345");
```

### Utility Functions

```typescript
import {
  uuidToBytes, // UUID string → 32-byte SHA-256
  fingerprintToBytes, // "sha256:hex..." → 32-byte array
  bytesToFingerprint, // 32-byte array → "sha256:hex..."
  proofTypeToU8, // "session_started" → 2
  u8ToProofType, // 2 → "session_started"
  deriveUserIndexPDA, // (pubkey, programId) → [PDA, bump]
  deriveProofPDA, // (proofId, programId) → [PDA, bump]
  deriveBatchPDA, // (batchId, programId) → [PDA, bump]
} from "@openlesson/proof-anchor-sdk";
```

## Integration with OpenLesson

This program is designed to replace the simulated anchoring in the OpenLesson agentic v2 API. The integration points are:

**Current (simulated):**

- `POST /api/v2/agent/proofs/:id/anchor` — stores a `sim_*` placeholder transaction signature
- `GET /api/v2/agent/proofs/:id/verify` — checks that anchor fields are present (no on-chain verification)

**With this program:**

1. Create `lib/agent-v2/solana.ts` — imports `ProofAnchorClient` from the SDK
2. Create `lib/agent-v2/solana-custodial.ts` — AES-256-GCM custodial wallet management using the `user_solana_wallets` table
3. Create `lib/agent-v2/solana-fee-payer.ts` — fee payer keypair loading and balance monitoring
4. Update the anchor endpoint to call `client.anchorProof()` / `client.anchorBatch()` and store the real `tx_signature`
5. Update the verify endpoint to call `client.verifyProofOnChain()` for on-chain fingerprint comparison

The database tables (`agent_proofs`, `agent_proof_batches`, `user_solana_wallets`) already exist in the OpenLesson schema (migration `026_agent_v2.sql`).

## Deployments

| Network     | Status           | Program ID                                     | Explorer                                                                                                            |
| ----------- | ---------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **Devnet**  | Live             | `6kFPmDutPLRigDcyKaLAtRnDbBZAh7teLfAMQLWhZJ5J` | [View on Explorer](https://explorer.solana.com/address/6kFPmDutPLRigDcyKaLAtRnDbBZAh7teLfAMQLWhZJ5J?cluster=devnet) |
| **Mainnet** | Not yet deployed | —                                              | —                                                                                                                   |

**Upgrade Authority:** `6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP` — only this wallet can deploy upgrades to the program.

### Deployment Phases

| Phase                 | Network                                 | Purpose                                                            |
| --------------------- | --------------------------------------- | ------------------------------------------------------------------ |
| **Phase 1**           | Localnet (`solana-test-validator`)      | Development and testing. No cost, instant transactions.            |
| **Phase 2** (current) | Devnet (`api.devnet.solana.com`)        | E2E testing with real network conditions. Free SOL via airdrop.    |
| **Phase 3**           | Mainnet (`api.mainnet-beta.solana.com`) | Production. Real proofs, permanent anchoring. OpenLesson pays gas. |

### Upgrading the Program

To deploy a new version (requires the upgrade authority keypair):

```bash
anchor build
solana program deploy target/deploy/openlesson_proof_anchor.so \
  --program-id 6kFPmDutPLRigDcyKaLAtRnDbBZAh7teLfAMQLWhZJ5J \
  --upgrade-authority <path-to-keypair.json> \
  --url devnet
```

### Running the Devnet Smoke Tests

```bash
npx ts-mocha -p tsconfig.json tests/devnet-smoke.ts --timeout 120000
```

Tests all 3 instructions against the live devnet deployment: `initialize_user_account`, `anchor_proof`, `anchor_batch`, plus error cases (invalid proof type, duplicate prevention).

### Environment Variables (for the OpenLesson API)

```bash
SOLANA_NETWORK=devnet                      # localnet | devnet | mainnet-beta
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_PROGRAM_ID=6kFPmDutPLRigDcyKaLAtRnDbBZAh7teLfAMQLWhZJ5J
SOLANA_FEE_PAYER_SECRET_KEY=<base64>       # Fee payer wallet secret key
SOLANA_WALLET_ENCRYPTION_KEY=<32-byte hex> # AES-256-GCM key for custodial wallets
```

## Program ID

```
6kFPmDutPLRigDcyKaLAtRnDbBZAh7teLfAMQLWhZJ5J
```

## License

MIT
