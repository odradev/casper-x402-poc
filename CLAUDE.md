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

Four workspace members implementing the x402 payment flow:

### `contract/` — Casper Smart Contract (Odra framework)
- `Cep18X402`: CEP-18 token extended with `transfer_with_authorization` — EIP-3009-style gasless transfers using off-chain signatures
- Authorization message is a 159-byte pre-image: `"casper-x402-v2:" || from(32) || to(32) || amount(U256 LE, 32) || valid_after(u64 LE, 8) || valid_before(u64 LE, 8) || nonce(32)`
- On-chain verification: replay protection via nonce mapping, time window check, public key → address derivation, Ed25519 signature verification
- Uses `odra` framework (v2.5.0, published crates)

### `x402-types/` — Shared Types
- Common request/response types: `PaymentRequired`, `CasperAuthorization`, `PaymentPayload`, `VerifyRequest`, `SettleRequest`, etc.

### `facilitator/` — HTTP Settlement Service (Axum)
- `GET /supported` — returns x402 protocol metadata (network, asset, version)
- `POST /verify` — off-chain signature and authorization validation
- `POST /settle` — verify then submit on-chain transfer
- Mirrors the contract's `build_message` for off-chain verification using `casper_types::crypto::verify`

### `demo/` — End-to-End Demo (Axum + reqwest)
- Runs a resource server, web UI, and client in one binary
- Resource server: `/api/data` (paid endpoint returning 402)
- Web UI: `/` (HTML interface) + `/api/run-flow` (triggers the payment flow)
- Flow: client GET → 402 with `X-PAYMENT-REQUIRED` header → client signs authorization → retries with `X-PAYMENT` header → resource server forwards to facilitator `/settle` → 200 with content

## Key Design Details

- The `build_message` function is duplicated in two places (contract, facilitator verify) — they must stay in sync
- Shared types live in the `x402-types` crate, used by both facilitator and demo
- All hex-encoded fields use raw bytes (no `0x` prefix); public keys and signatures include the Casper tag byte
- `block_time()` in Odra returns milliseconds; the contract divides by 1000 for seconds

## Environment Configuration

Copy `.env.example` to `.env`. Key variables:
- `ODRA_CASPER_LIVENET_*` — Casper node connection settings
- `CONTRACT_PACKAGE_HASH` — deployed contract address (set automatically by the Docker deployer)
- `SECRET_KEY_PATH` — path to the Ed25519 secret key for the demo client
