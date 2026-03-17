# Casper x402 — Proof of Concept

A proof-of-concept implementation of the [x402 payment protocol](https://www.x402.org/) on the [Casper blockchain](https://casper.network/). Enables HTTP 402-based micropayments using EIP-3009-style `transfer_with_authorization` on CEP-18 tokens.

## What is x402?

The [x402 protocol](https://www.x402.org/) brings the long-dormant HTTP 402 ("Payment Required") status code to life. It defines a standard flow for machine-to-machine payments on the web:

1. A client requests a paid resource.
2. The server responds with **402** and a payment requirement header describing how to pay.
3. The client signs a payment authorization off-chain and retries the request with a payment header.
4. The server verifies the payment (optionally settling on-chain) and returns the resource.

This project adapts the protocol for the Casper network by implementing gasless, signature-based token transfers — similar to [EIP-3009](https://eips.ethereum.org/EIPS/eip-3009) (`transferWithAuthorization`) — on top of the CEP-18 token standard.

## Architecture

```
┌──────────┐   GET /resource   ┌──────────────────┐   POST /settle   ┌──────────────┐
│          │ ────────────────▶ │                  │ ───────────────▶ │              │
│  Client  │   ◀──── 402 ────  │  Resource Server │  ◀── 200 ──────  │  Facilitator │
│          │                   │                  │                  │              │
│          │   GET + X-PAYMENT │                  │                  │              │
│          │ ────────────────▶ │                  │                  │              │
│          │   ◀──── 200 ───── │                  │                  │              │
└──────────┘                   └──────────────────┘                  └──────┬───────┘
                                                                            │
                                                                     on-chain settle
                                                                            │
                                                                     ┌──────▼───────┐
                                                                     │   Casper     │
                                                                     │   Network    │
                                                                     │  (CEP-18     │
                                                                     │   + x402)    │
                                                                     └──────────────┘
```

The project consists of four workspace members:

### `contract/` — Smart Contract (CEP-18 + x402)

A CEP-18 token contract extended with `transfer_with_authorization` — gasless, off-chain-signed transfers. Built with the [Odra](https://github.com/odradev/odra) smart contract framework.

- **Authorization pre-image**: `"casper-x402-v2:" || from || to || amount || valid_after || valid_before || nonce`
- **On-chain verification**: replay protection (nonce mapping), time-window checks, Ed25519 signature verification
- **Standard CEP-18 interface**

### `x402-types/` — Shared Types

Common request/response types used by both the facilitator and demo crates: `PaymentRequired`, `CasperAuthorization`, `PaymentPayload`, `VerifyRequest`, `SettleRequest`, and related structs.

### `facilitator/` — Settlement Service

An HTTP service (Axum) that verifies payment authorizations and settles them on-chain.

| Endpoint | Description |
|---|---|
| `GET /supported` | Returns x402 protocol metadata (network, asset, version) |
| `POST /verify` | Off-chain signature and authorization validation |
| `POST /settle` | Verify + submit on-chain `transfer_with_authorization` |

Settles payments against a local Casper network (nctl).

### `demo/` — End-to-End Demo

A self-contained binary that runs a resource server, a web UI, and a client, demonstrating the full x402 payment flow. The resource server exposes a paid endpoint at `/api/data`, and the UI at `/` provides a browser-based interface to trigger the payment flow via `/api/run-flow`.

## Prerequisites

- **Rust nightly** (`nightly-2025-01-01`, pinned in `rust-toolchain`)
- **Docker & Docker Compose** (for running a local Casper network)
- **[just](https://github.com/casey/just)** command runner (optional, but convenient)

## Quick Start

### 1. Clone and configure

```bash
git clone https://github.com/odradev/casper-x402-poc.git
cd casper-x402-poc
cp .env.example .env
```

### 2. Build the contract

The WASM binary must be built before starting the Docker stack, as the `wasm/` directory is mounted as a volume:

```bash
just build-contract
```

### 3. Run with Docker

Spin up a local Casper network, deploy the contract, and start the facilitator:

```bash
just docker-up

# Follow logs
just docker-logs {{service}}

# Check service status
just docker-ps
```

This starts:
- **nctl** — local Casper test network
- **deployer** — deploys the CEP-18 x402 contract (one-shot)
- **facilitator** — settlement service on port 3001

### 4. Run the demo

Once the facilitator is running:

```bash
just run-demo
```

The demo uses keys from the local nctl network, starts a resource server, and runs through the complete payment flow against the local Casper node.

## Configuration

Copy `.env.example` to `.env` and adjust as needed:

| Variable | Description | Default |
|---|---|---|
| `PORT` | Facilitator listen port | `3001` |
| `RESOURCE_SERVER_PORT` | Demo resource server port | `3002` |
| `RESOURCE_SERVER_URL` | URL the demo client uses to reach the resource server | `http://127.0.0.1:3002` |
| `FACILITATOR_URL` | URL the resource server uses to reach the facilitator | `http://127.0.0.1:3001` |
| `CONTRACT_PACKAGE_HASH` | Deployed contract address (set automatically by the deployer) | — |
| `PAY_TO` | Recipient account hash | — |
| `PAYMENT_AMOUNT` | Payment amount in token units | `1000000` |
| `SECRET_KEY_PATH` | Path to the Ed25519 secret key for the demo client | `.node_keys/secret_key.pem` |
| `ODRA_CASPER_LIVENET_*` | Casper node connection settings | — |

## Development

```bash
# Run contract tests
just test

# Run a specific test
just test-one transfer_with_authorization_succeeds

# Build contract WASM
just build-contract

# Run facilitator + demo together
just run-all

# Docker helpers
just docker-up
just docker-down
just docker-logs facilitator
```

## Resources

- **[x402 Protocol Specification](https://www.x402.org/)** — the payment protocol this project implements
- **[EIP-3009: Transfer With Authorization](https://eips.ethereum.org/EIPS/eip-3009)** — the Ethereum standard that inspired the `transfer_with_authorization` pattern
- **[Casper Network](https://casper.network/)** — the L1 blockchain
- **[CEP-18 Token Standard](https://github.com/casper-ecosystem/cep18)** — Casper's fungible token standard (analogous to ERC-20)
- **[Odra Framework](https://github.com/odradev/odra)** — smart contract framework used to build the contract
- **[Odra Documentation](https://odra.dev/docs)** — Odra guides and API reference

## License

See [LICENSE](LICENSE) for details.
