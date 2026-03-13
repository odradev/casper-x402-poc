#!/usr/bin/env bash
set -euo pipefail

NCTL_ASSETS=/nctl-assets
KEYS_DIR=/keys
DEPLOYED_DIR=/deployed

echo "[deployer] Copying user secret keys..."
mkdir -p "$KEYS_DIR/user-1" "$KEYS_DIR/user-2"
cp "$NCTL_ASSETS/net-1/users/user-1/secret_key.pem" "$KEYS_DIR/user-1/secret_key.pem"
cp "$NCTL_ASSETS/net-1/users/user-2/secret_key.pem" "$KEYS_DIR/user-2/secret_key.pem"
echo "[deployer] Keys copied to $KEYS_DIR"

# Use user-1 as the deployer account
export ODRA_CASPER_LIVENET_SECRET_KEY_PATH="$KEYS_DIR/user-1/secret_key.pem"

echo "[deployer] Deploying contract (working dir: $(pwd))..."
cli deploy

echo "[deployer] Deployment complete. deployed_contracts.toml:"
cat deployed_contracts.toml

# Extract the package hash written by odra-cli.
# The TOML has a line like:  package_hash = "hash-abcdef..."
PACKAGE_HASH=$(grep -oP '(?<=package_hash = ")[^"]+' deployed_contracts.toml | head -1)

if [ -z "$PACKAGE_HASH" ]; then
    echo "[deployer] ERROR: could not extract package_hash from deployed_contracts.toml" >&2
    cat deployed_contracts.toml >&2
    exit 1
fi

mkdir -p "$DEPLOYED_DIR"
echo -n "$PACKAGE_HASH" > "$DEPLOYED_DIR/contract_package_hash"
echo "[deployer] Contract package hash saved: $PACKAGE_HASH"
