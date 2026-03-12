# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Proof-of-concept implementation of the [x402 payment protocol](https://www.x402.org/) on the Casper blockchain. Implements HTTP 402-based micropayments using EIP-3009-style `transfer_with_authorization` on CEP-18 tokens.

## Build & Test Commands

```bash
# Build entire workspace
cargo build

# Build only the smart contract
cargo build -p cep18-x402

# Run contract tests (uses Odra mock VM, no live node needed)
cargo test -p cep18-x402

# Run a single test
cargo test -p cep18-x402 -- transfer_with_authorization_succeeds

# Build the WASM contract (requires wasm32 target)
cargo run -p cep18-x402 --bin cep18_x402_build_contract

# Run the facilitator server
cargo run -p facilitator

# Run the demo (starts resource server + client flow)
cargo run -p demo
```

Requires Rust nightly (`nightly-2025-01-01`, pinned in `rust-toolchain`).

## Architecture

Three workspace members implementing the x402 payment flow:

### `contract/` — Casper Smart Contract (Odra framework)
- `Cep18X402`: CEP-18 token extended with `transfer_with_authorization` — EIP-3009-style gasless transfers using off-chain signatures
- Authorization message is a 159-byte pre-image: `"casper-x402-v1:" || from(32) || to(32) || amount(U256 LE, 32) || valid_after(u64 LE, 8) || valid_before(u64 LE, 8) || nonce(32)`
- On-chain verification: replay protection via nonce mapping, time window check, public key → address derivation, Ed25519 signature verification
- Uses `odra` framework with workspace dependencies pointing to local `../odra/` checkout

### `facilitator/` — HTTP Settlement Service (Axum)
- `POST /verify` — off-chain signature and authorization validation
- `POST /settle` — verify then submit on-chain transfer (or mock)
- `CasperSettler` — on-chain settlement stub (live settlement not yet implemented; use `MOCK_MODE=true`)
- Mirrors the contract's `build_message` for off-chain verification using `casper_types::crypto::verify`

### `demo/` — End-to-End Demo (Axum + reqwest)
- Runs both a resource server and a client in one binary
- Flow: client GET → 402 with `X-PAYMENT-REQUIRED` header → client signs authorization → retries with `X-PAYMENT` header → resource server forwards to facilitator `/settle` → 200 with content
- Generates ephemeral Ed25519 keys; no external setup required

## Key Design Details

- The `build_message` function is duplicated in three places (contract, facilitator verify, demo client) — they must stay in sync
- Types (`PaymentRequired`, `CasperAuthorization`, `PaymentPayload`) are duplicated between `facilitator/src/types.rs` and `demo/src/types.rs`
- All hex-encoded fields use raw bytes (no `0x` prefix); public keys and signatures include the Casper tag byte
- `block_time()` in Odra returns milliseconds; the contract divides by 1000 for seconds
- The Odra dependency is a local path (`../odra/`) — the sibling `odra` repo must be checked out

## Environment Configuration

Copy `.env.example` to `.env`. Key variables:
- `MOCK_MODE=true` — facilitator returns fake tx hashes without hitting a Casper node
- `ODRA_CASPER_LIVENET_*` — Casper node connection settings (only needed when `MOCK_MODE=false`)
- `CONTRACT_PACKAGE_HASH` — deployed contract address (needed for live settlement)
