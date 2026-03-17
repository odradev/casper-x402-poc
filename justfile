# Build the WASM contract (requires wasm32 target)
build-contract:
    cargo odra build

# Run contract tests
test:
    cargo test -p cep18-x402

# Run a single contract test by name
test-one name:
    cargo test -p cep18-x402 -- {{name}}

# Run the demo (resource server + client flow)
run-demo: copy-node-keys
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

# Start the full Docker stack (nctl + deployer + facilitator)
docker-up:
    docker compose up --build -d

# Stop all Docker services
docker-down:
    docker compose down

# Stop all Docker services and remove volumes
docker-clean:
    docker compose down -v

# Show Docker service logs (follow mode)
docker-logs *args:
    docker compose logs -f {{args}}

# Show status of Docker services
docker-ps:
    docker compose ps

# Restart a specific Docker service
docker-restart service:
    docker compose restart {{service}}

copy-node-keys:
    docker exec casper-x402-poc-nctl-1 cat /home/casper/casper-nctl/assets/net-1/users/user-1/secret_key.pem > .node_keys/secret_key.pem