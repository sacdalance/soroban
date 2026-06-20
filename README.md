# Private Nanopayments

Unlinkable USDC transfers on Stellar, powered by ZK proofs.

## Problem

Migrants, NGOs, and privacy-conscious senders cannot move USDC on Stellar without permanently exposing the sender–recipient link: every standard transfer is trivially traceable on the public ledger.

## Solution

A Soroban contract escrows USDC against an opaque commitment `C = Poseidon(secret, salt)` at deposit time; the secret is delivered off-chain. The recipient later submits a ZK proof plus a nullifier `N` derived from the secret to claim the funds to any address. Deposit and claim appear as unrelated transactions on-chain — the only link is the secret, which never touches the chain.

## Hackathon Context

Built for **Stellar Bootcamp 2026**. Stellar Protocol 25 ("X-Ray") adds native **BN254** elliptic curve operations and **Poseidon** hashing as host functions, making on-chain **Groth16** verification feasible inside Soroban for the first time. ZK is load-bearing here, not decorative — without it the privacy property collapses.

## Architecture

```
Sender ──deposit(from, amount, C)──► Contract  (C = Poseidon(secret, salt))
                                         │
                          secret delivered off-chain (email / SMS / QR)
                                         │
Recipient ◄──claim(proof, C, N, dst)── Contract  (Groth16 proof + nullifier N)
```

- **Deposit** escrows USDC under `C`; reveals nothing about the recipient.
- **Claim** verifies a proof binding `C` to `N`, checks `N` is unused, then pays out to any destination address.
- An on-chain observer sees two unrelated transactions.

## Stellar Features Used

| Feature | Purpose |
|---|---|
| Soroban smart contracts | Trustless escrow + proof verification |
| USDC SAC (Stellar Asset Contract) | Native USDC transfers without wrapping |
| BN254 precompile | Groth16 pairing checks on-chain (Protocol 25) |
| Poseidon precompile | Commitment hashing + nullifier derivation (Protocol 25) |

## Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target
- [`soroban-cli`](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) (compatible with Protocol 25 / Soroban SDK v25)
- A funded Stellar Testnet account

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Test

```bash
cargo test
```

## Deploy (Testnet)

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/private_nanopayments.wasm \
  --source <YOUR_TESTNET_IDENTITY> \
  --network testnet
```

Then initialize with the testnet USDC SAC address:

```bash
soroban contract invoke \
  --id <CONTRACT_ID> --source <YOUR_TESTNET_IDENTITY> --network testnet \
  -- initialize --token <USDC_SAC_ADDRESS>
```

## Sample Invocations

```bash
# Deposit 500 USDC against a commitment
soroban contract invoke \
  --id <CONTRACT_ID> --source <SENDER_IDENTITY> --network testnet \
  -- deposit --from <SENDER_ADDRESS> --amount 500 \
  --commitment 0707070707070707070707070707070707070707070707070707070707070707

# Claim with a proof, the original commitment, and a fresh nullifier
soroban contract invoke \
  --id <CONTRACT_ID> --source <ANY_RELAYER_IDENTITY> --network testnet \
  -- claim --proof 01 \
  --commitment 0707070707070707070707070707070707070707070707070707070707070707 \
  --nullifier 0909090909090909090909090909090909090909090909090909090909090909 \
  --recipient <RECIPIENT_ADDRESS>
```

## Security Notes

- Nullifier uniqueness is checked **before** any proof verification or transfer (double-spend prevention).
- State (`used_nullifiers`, cleared commitment) is mutated **before** the token transfer (checks-effects-interactions).
- `deposit` requires `from.require_auth()`; `claim` deliberately allows any caller to relay a valid proof on behalf of the recipient.
- The Groth16 verifier (`verify_proof`) is currently a **placeholder stub** — see the `// TODO` in `lib.rs`. It must be replaced with real BN254/Poseidon verification (Protocol 25 host functions) before any non-testnet use.
- The USDC token address is never hardcoded; it is set once via `initialize` and read from contract storage.
- No `unsafe` code; this project has **not undergone a trusted-setup ceremony** and is testnet-only.

## License

MIT
