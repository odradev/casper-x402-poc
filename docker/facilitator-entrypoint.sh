#!/usr/bin/env bash
set -euo pipefail

DEPLOYED_DIR=/deployed
KEYS_DIR=/keys

# Read contract address written by the deployer
CONTRACT_ADDR=$(cat "$DEPLOYED_DIR/contract_package_hash")
echo "[facilitator] Using contract address: $CONTRACT_ADDR"

export X402_TOKEN_ADDRESS="$CONTRACT_ADDR"
export ODRA_CASPER_LIVENET_SECRET_KEY_PATH="$KEYS_DIR/user-1/secret_key.pem"

exec /usr/local/bin/facilitator
