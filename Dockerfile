# ─── Stage 1: builder ────────────────────────────────────────────────────────
FROM rust:bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev librdkafka-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY x402-types/ x402-types/
COPY x402-eip712/ x402-eip712/
COPY contract/ contract/
COPY facilitator/ facilitator/
COPY demo/ demo/
# COPY contract/wasm/ wasm/

RUN cargo +nightly build --release \
    --bin deployer \
    --bin facilitator

# ─── Stage 2: runtime ─────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 librdkafka1 curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# project_root crate searches upward for Cargo.lock to determine project root.
# Without it, find_wasm_file_path fails silently (no log output at all).
COPY --from=builder /app/Cargo.lock /app/Cargo.lock

COPY --from=builder /app/target/release/deployer /usr/local/bin/deployer
COPY --from=builder /app/target/release/facilitator /usr/local/bin/facilitator
