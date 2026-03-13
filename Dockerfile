# ─── Stage 1: builder ────────────────────────────────────────────────────────
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Pin nightly toolchain (matches rust-toolchain file)
RUN rustup toolchain install nightly-2025-01-01 \
    && rustup default nightly-2025-01-01

WORKDIR /src

# Clone odra at the exact version used by the workspace (path = "../odra/...")
RUN git clone --depth 1 --branch release/2.6.0 https://github.com/odradev/odra.git odra

# Copy project source
COPY . casper-x402-poc/

WORKDIR /src/casper-x402-poc

# Cache dependencies first (dummy build)
RUN cargo fetch

# Build the CLI deployer binary (cep18-x402 crate)
RUN cargo build --release -p cep18-x402 --bin cli

# Build the facilitator binary
RUN cargo build --release -p facilitator


# ─── Stage 2: deployer ───────────────────────────────────────────────────────
FROM debian:bookworm-slim AS deployer

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/casper-x402-poc/target/release/cli /usr/local/bin/cli

# Odra CLI needs Odra.toml + wasm/ next to each other when deploying.
# It also uses project-root crate (needs Cargo.toml) and writes to resources/.
RUN mkdir -p /app/contract/wasm /app/contract/resources
COPY contract/Odra.toml    /app/contract/Odra.toml
COPY contract/wasm/Cep18X402.wasm /app/contract/wasm/Cep18X402.wasm
COPY --from=builder /src/casper-x402-poc/Cargo.lock /app/contract/Cargo.lock

COPY docker/deployer-entrypoint.sh /usr/local/bin/deployer-entrypoint.sh
RUN chmod +x /usr/local/bin/deployer-entrypoint.sh

WORKDIR /app/contract
ENTRYPOINT ["/usr/local/bin/deployer-entrypoint.sh"]


# ─── Stage 3: facilitator ────────────────────────────────────────────────────
FROM debian:bookworm-slim AS facilitator

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/casper-x402-poc/target/release/facilitator /usr/local/bin/facilitator

COPY docker/facilitator-entrypoint.sh /usr/local/bin/facilitator-entrypoint.sh
RUN chmod +x /usr/local/bin/facilitator-entrypoint.sh

EXPOSE 3001
ENTRYPOINT ["/usr/local/bin/facilitator-entrypoint.sh"]
