# Build the WASM contract (requires wasm32 target)
build-contract:
    cd contract && cargo odra build

# Build the facilitator server
build-facilitator:
    cargo build -p facilitator --release

# Build the demo
build-demo:
    cargo build -p demo

# Run contract tests
test:
    cargo test -p cep18-x402

# Run a single contract test by name
test-one name:
    cargo test -p cep18-x402 -- {{name}}

# Run the facilitator server
run-facilitator:
    cargo run -p facilitator

# Run the demo (resource server + client flow)
run-demo:
    cargo run -p demo

# Run facilitator and demo together (facilitator in background)
run-all:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Starting facilitator..."
    cargo run -p facilitator &
    FACILITATOR_PID=$!
    # Give the facilitator a moment to start
    sleep 2
    echo "Starting demo..."
    cargo run -p demo
    kill $FACILITATOR_PID 2>/dev/null || true

# Copy .env.example to .env (won't overwrite existing)
setup:
    @[ -f .env ] && echo ".env already exists, skipping" || cp .env.example .env && echo "Created .env from .env.example"
