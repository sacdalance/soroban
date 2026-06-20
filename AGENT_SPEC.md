# 🛡️ AGENT SPEC — Private Nanopayments on Stellar

> **For:** AI coding agent (Antigravity / Cursor / Copilot Workspace)
> **Status:** Authoritative source of truth — do not override without explicit user approval
> **Security level:** HIGH — all agent outputs must satisfy the security constraints in §6

---

## 1. Project Identity

| Field | Value |
|---|---|
| **Name** | Private Nanopayments |
| **Tagline** | Unlinkable USDC transfers on Stellar, powered by ZK proofs |
| **License** | MIT |
| **Network** | Stellar (Protocol 25+) |
| **Language** | Rust (Soroban SDK `v25`) |
| **Edition** | 2021 |

---

## 2. Problem & Solution

### 2.1 Problem

Migrants, NGOs, and privacy-conscious senders **cannot move USDC on Stellar** without permanently exposing the sender–recipient link on a fully public ledger. Every standard USDC transfer is trivially traceable: the sender's account, recipient's account, and amount are permanently indexed on-chain.

### 2.2 Solution

A Soroban smart contract that severs the on-chain sender–recipient link using a **commit-reveal + ZK proof** scheme:

```
Sender  ──deposit(USDC, C)──►  Contract  (C = Poseidon(secret, salt))
                                   │
                     secret delivered off-chain (email / QR code / SMS)
                                   │
Recipient  ◄──claim(proof, N, dst)──  Contract  (Groth16 proof + nullifier N)
```

- **Deposit** and **Claim** are unrelated transactions to any on-chain observer.
- The **nullifier** `N = Poseidon(secret)` prevents double-spending without revealing the secret.
- **No wallet pre-required** for recipients: destination address is provided at claim time.

### 2.3 Why Stellar Protocol 25

Protocol 25 introduces **native BN254 elliptic curve** and **Poseidon hash** host functions, making on-chain **Groth16 verification feasible inside Soroban** for the first time. This is the load-bearing technical enabler — ZK is not decorative here.

| Stellar Feature Used | Purpose |
|---|---|
| Soroban smart contracts | Trustless escrow + proof verification |
| USDC SAC (Stellar Asset Contract) | Native USDC transfers without wrapping |
| BN254 precompile | Groth16 pairing checks on-chain |
| Poseidon precompile | Commitment hashing + nullifier derivation |

---

## 3. Target Users

| User | Need |
|---|---|
| Migrants | Send remittances without exposing financial relationships |
| NGOs | Distribute grants to recipients without surveillance risk |
| Privacy-conscious individuals | Move funds without permanent ledger correlation |

Recipients are identified **only by email or phone** — no wallet required at deposit time.

---

## 4. Core Flow (Step-by-Step)

```
Step 1 — SENDER
  Action : deposit(from, amount, commitment C)
  Result : Contract escrows USDC; stores C in `commitments` map
  On-chain: deposit tx visible; recipient unknown

Step 2 — SYSTEM (off-chain, mocked in tests)
  Action : deliver secret S off-chain (email / SMS / QR code)
  Result : No on-chain trace whatsoever

Step 3 — RECIPIENT
  Action : claim(proof π, nullifier N, destination addr)
  Result : Contract verifies Groth16 π; checks N unused;
           marks N in `used_nullifiers` set; transfers USDC to destination

Step 4 — OBSERVER
  View   : Block explorer shows deposit tx and claim tx as unrelated
  Link   : Cryptographically severed — commitment binds secret, not identity
```

---

## 5. Output Files

The agent **must produce exactly these four files** with the specifications below.

### 5.1 `contracts/private_nanopayments/src/lib.rs`

**Contract storage:**
- `commitments: Map<BytesN<32>, u128>` — maps commitment hash → escrowed amount
- `used_nullifiers: Set<BytesN<32>>` — set of spent nullifiers

**Public functions:**

| Function | Signature | Behaviour |
|---|---|---|
| `deposit` | `(from: Address, amount: u128, commitment: BytesN<32>)` | Authorise `from`; transfer `amount` USDC from `from` to contract; store `commitment → amount`; emit `Deposit` event |
| `claim` | `(proof: Bytes, nullifier: BytesN<32>, recipient: Address)` | Reject if `nullifier` already used; verify Groth16 proof (BN254/Poseidon); look up amount; mark nullifier used; transfer USDC to `recipient`; emit `Claim` event |

**Rules:**
- Every function **must have a one-line `// Why:` comment** explaining its security purpose.
- Use `soroban_sdk::token::Client` for USDC transfers (never raw ledger manipulation).
- Panic with descriptive messages on every rejection path.
- No `unsafe` blocks.
- No `.unwrap()` without a preceding bounds/existence check (use `expect("...")` with context).

### 5.2 `contracts/private_nanopayments/src/test.rs`

Exactly **5 tests** under `#[cfg(test)] mod tests`, using `Env::default()` and `soroban-sdk` testutils:

| # | Name | Scenario |
|---|---|---|
| 1 | `test_happy_path_deposit_then_claim` | Full end-to-end: deposit → claim → USDC received |
| 2 | `test_replayed_nullifier_rejected` | Second claim with same nullifier must panic |
| 3 | `test_storage_updated_post_claim` | After claim: nullifier in set, commitment amount zeroed |
| 4 | `test_invalid_proof_rejected` | Malformed/incorrect Groth16 proof must panic |
| 5 | `test_unauthorized_deposit_rejected` | Deposit without `from.require_auth()` must panic |

### 5.3 `contracts/private_nanopayments/Cargo.toml`

```toml
[package]
name    = "private_nanopayments"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
soroban-sdk = { workspace = true }

[dev-dependencies]
soroban-sdk = { workspace = true, features = ["testutils"] }

[profile.release]
opt-level = "z"
overflow-checks = true
debug = 0
strip = "symbols"
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = true
```

### 5.4 `README.md` (project root — replace existing)

Sections required (in order):

1. **Name + Tagline**
2. **Problem / Solution** (2–3 sentences each)
3. **Hackathon Context** — Stellar Bootcamp 2026; why Protocol 25 matters
4. **Architecture diagram** (ASCII or Mermaid)
5. **Stellar Features Used** (table)
6. **Prerequisites** — Rust stable, `soroban-cli`, Stellar Testnet account
7. **Build** — `cargo build --target wasm32-unknown-unknown --release`
8. **Test** — `cargo test`
9. **Deploy** — `soroban contract deploy` with testnet flags
10. **Sample `soroban contract invoke` calls** for `deposit` and `claim`
11. **Security Notes** (see §6)
12. **License** — MIT

---

## 6. Security Constraints ⚠️

> These are **non-negotiable**. The agent must enforce all of them.

### 6.1 Nullifier Uniqueness (Double-Spend Prevention)
- The `used_nullifiers` set **must be checked before any fund transfer**.
- Order: `check nullifier → verify proof → mark used → transfer`. Never reorder.
- If the nullifier check is omitted or placed after transfer, the implementation is **invalid**.

### 6.2 Commitment Binding
- Commitment `C = Poseidon(secret, salt)` must be stored as-is.
- Do **not** hash it again on-chain — the contract stores the raw commitment submitted by the sender.
- The proof verifier binds the nullifier to the commitment; any mismatch must fail verification.

### 6.3 Proof Verification
- Groth16 verification must use the **BN254 host function** from `soroban-sdk` (Protocol 25).
- A placeholder `verify_proof(proof, nullifier, commitment) → bool` stub is acceptable in early iterations, but **must be clearly marked `// TODO: replace with real BN254 Groth16 verify`**.
- Never skip or mock the nullifier uniqueness check even when the proof verifier is stubbed.

### 6.4 Authorization
- `deposit()` **must** call `from.require_auth()` before any state mutation.
- `claim()` does **not** require the recipient to be the caller — by design, anyone can submit a valid proof on behalf of a recipient.

### 6.5 Reentrancy / Ordering
- State mutation (`used_nullifiers.insert`) **must occur before** the token transfer call.
- This follows the checks-effects-interactions pattern to prevent reentrancy-style re-entrancy via token callbacks.

### 6.6 No Hardcoded Secrets
- The USDC token contract address **must be passed as a constructor/init parameter** or read from storage — never hardcoded.
- No private keys, mnemonics, or API secrets in any source file.

### 6.7 No `unsafe` Code
- Zero `unsafe` blocks in `lib.rs` or `test.rs`.

### 6.8 Input Validation
- `amount` must be `> 0`; reject with a clear panic message if not.
- `commitment` and `nullifier` must be exactly 32 bytes (enforced by `BytesN<32>` type).

---

## 7. Key Risk & Fallback

| Risk | Mitigation |
|---|---|
| Soroban instruction limit for Groth16 | Benchmark verifier first; fallback to PLONK (smaller proof) or off-chain verify + on-chain attestation |
| Secret delivery failure | QR-code delivery as offline / low-connectivity fallback |
| Nullifier front-running | Use commit-reveal for claim submission (future iteration) |
| Testnet USDC not available | Use a mock SAC token in tests; document real USDC SAC address for mainnet |

---

## 8. Reference URLs

| Purpose | URL |
|---|---|
| Stellar Bootcamp 2026 (deploy guide) | https://github.com/armlynobinguar/Stellar-Bootcamp-2026 |
| Community Treasury (full-stack example) | https://github.com/armlynobinguar/community-treasury |
| Soroban SDK Docs | https://docs.rs/soroban-sdk/latest/soroban_sdk/ |
| Protocol 25 BN254/Poseidon Host Fns | https://developers.stellar.org/docs/learn/encyclopedia/contract-development/cryptography |

---

## 9. Agent Task Checklist

When implementing this spec, the agent must complete these steps **in order**:

- [ ] Create `contracts/private_nanopayments/` directory structure
- [ ] Write `Cargo.toml` for the new contract (§5.3)
- [ ] Add `private_nanopayments` to the workspace `Cargo.toml` `members` list
- [ ] Implement `lib.rs` with `deposit` + `claim` per §5.1 and all §6 constraints
- [ ] Implement `test.rs` with exactly 5 tests per §5.2
- [ ] Replace root `README.md` with full documentation per §5.4
- [ ] Run `cargo build --target wasm32-unknown-unknown --release` — must succeed
- [ ] Run `cargo test` — all 5 tests must pass
- [ ] Verify no `unsafe` blocks exist (`grep -r 'unsafe' contracts/private_nanopayments/src/`)
- [ ] Verify no hardcoded USDC addresses or secrets in source files

---

## 10. Glossary

| Term | Definition |
|---|---|
| **Commitment** | `C = Poseidon(secret, salt)` — hash stored on-chain at deposit; binds sender to secret without revealing it |
| **Nullifier** | `N = Poseidon(secret)` — unique tag derived from secret; marks a commitment as spent |
| **Groth16** | Succinct ZK-SNARK proving scheme; verifier runs in O(1) on-chain using BN254 pairings |
| **BN254** | Pairing-friendly elliptic curve supported natively in Stellar Protocol 25 |
| **Poseidon** | ZK-friendly hash function supported natively in Stellar Protocol 25 |
| **SAC** | Stellar Asset Contract — Soroban interface for native Stellar assets including USDC |
| **Nullifier front-running** | An attack where an observer copies a valid proof and submits it before the intended recipient |

---

*Generated by Antigravity · Conversation `14c8cd52-ed99-4342-8e15-308138a36f43` · 2026-06-20*
