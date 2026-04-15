#!/usr/bin/env bash
set -euo pipefail

CONTAINER_NAME="ephemera"
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# cleanup() {
#   sudo nixos-container stop "$CONTAINER_NAME" 2>/dev/null || true
#   sudo nixos-container destroy "$CONTAINER_NAME" 2>/dev/null || true
# }
# trap cleanup EXIT

# Check required env vars
for var in LLM_BASE_URL LLM_MODEL LLM_API_KEY; do
  if [ -z "${!var:-}" ]; then
    echo "ERROR: $var is not set"
    exit 1
  fi
done

# 1. Resolve flake source and nixpkgs to /nix/store paths (shared with container)
FLAKE_SOURCE=$(nix flake metadata --json "$REPO_ROOT" | jq -r '.path')
NIXPKGS_PATH=$(nix flake archive --json "$REPO_ROOT" | jq -r '.inputs.nixpkgs.path')

echo "FLAKE_SOURCE=$FLAKE_SOURCE"
echo "NIXPKGS_PATH=$NIXPKGS_PATH"

# 2. Pre-build packages on host
echo "Building ephemera-ai packages..."
nix build "$REPO_ROOT#default" -L

# 3. Create & start container
echo "Creating NixOS container..."
sudo nixos-container create "$CONTAINER_NAME" --config-file "$REPO_ROOT/nix/integration-test/configuration.nix"

echo "Starting container..."
sudo nixos-container start "$CONTAINER_NAME"
sleep 3

# 4. Copy templates/default into container
echo "Copying templates/default into container..."
sudo nixos-container run "$CONTAINER_NAME" -- mkdir -p /home/ephemera
sudo nixos-container run "$CONTAINER_NAME" -- cp --no-preserve=mode -r "$FLAKE_SOURCE/templates/default" /home/ephemera/config
sudo nixos-container run "$CONTAINER_NAME" -- chown -R 1000:1000 /home/ephemera/config

# 5. Patch config for integration test (LLM credentials + direct MySQL)
sudo nixos-container run "$CONTAINER_NAME" -- su - ephemera -c '
  # LLM credentials
  sed -i "s|base_url = .*|base_url = \"'"$LLM_BASE_URL"'\";|" /home/ephemera/config/ephemera-ai.nix
  sed -i "s|model = .*|model = \"'"$LLM_MODEL"'\";|" /home/ephemera/config/ephemera-ai.nix
  sed -i "s|api_key = .*|api_key = \"'"$LLM_API_KEY"'\";|" /home/ephemera/config/ephemera-ai.nix

  # Remove Podman mysql instances block
  sed -i "/# MySQL instances/,/^  };/d" /home/ephemera/config/ephemera-ai.nix

  # Loom: use direct MySQL connection
  sed -i "s|mysql = \"loom-mysql\";|mysql = null; mysql_url = \"mysql://epha:integration-test-pass@localhost:3306/psyche_loom\";|" /home/ephemera/config/ephemera-ai.nix

  # Atrium: use direct MySQL connection
  sed -i "s|mysql = \"atrium-mysql\";|mysql = null; mysql_url = \"mysql://epha:integration-test-pass@localhost:3306/dialogue_atrium\";|" /home/ephemera/config/ephemera-ai.nix

  # Replace flake inputs with local /nix/store paths
  sed -i "s|url = \"github:ImitationGameLabs/ephemera-ai\";|url = \"path:'"$FLAKE_SOURCE"'\";|" /home/ephemera/config/flake.nix
  sed -i "s|url = \"github:nixos/nixpkgs/nixos-unstable\";|url = \"path:'"$NIXPKGS_PATH"'\";|" /home/ephemera/config/flake.nix

  # Inject integration-test grounding append (the brief-existence philosophical framing)
  sed -i "s|prompt_append_file = null;|prompt_append_file = \"'"$FLAKE_SOURCE"'/crates/epha-ai/prompts/integration-test-append.md\";|" /home/ephemera/config/ephemera-ai.nix
'

# 6. Run home-manager switch
# echo "Running home-manager switch..."
sudo nixos-container run "$CONTAINER_NAME" -- su - ephemera -c "
  cd /home/ephemera/config &&
  home-manager switch --flake .
"

# # 5. Wait for services
# wait_for_health() {
#   local name=$1 port=$2 max_wait=$3 elapsed=0
#   echo "Waiting for $name..."
#   while [ "$elapsed" -lt "$max_wait" ]; do
#     if curl -sf "http://localhost:$port/health" > /dev/null 2>&1; then
#       echo "$name is ready (${elapsed}s)"
#       return 0
#     fi
#     sleep 2
#     elapsed=$((elapsed + 2))
#   done
#   echo "ERROR: $name not ready after ${max_wait}s"
#   return 1
# }

# wait_for_health "agora"  3000 60  || exit 1
# wait_for_health "kairos" 3003 60  || exit 1
# wait_for_health "loom"   3001 120 || exit 1
# wait_for_health "atrium" 3002 120 || exit 1

# # 6. Integration tests
# PASSED=0
# FAILED=0

# for svc in agora:3000 loom:3001 atrium:3002 kairos:3003; do
#   name="${svc%%:*}"; port="${svc##*:}"
#   if curl -sf "http://localhost:$port/health" > /dev/null; then
#     echo "PASS: $name"
#     PASSED=$((PASSED + 1))
#   else
#     echo "FAIL: $name"
#     FAILED=$((FAILED + 1))
#   fi
# done

# if sudo nixos-container run "$CONTAINER_NAME" -- \
#   su - ephemera -c "systemctl --user is-active epha-ai" > /dev/null 2>&1; then
#   echo "PASS: epha-ai"
#   PASSED=$((PASSED + 1))
# else
#   echo "FAIL: epha-ai"
#   FAILED=$((FAILED + 1))
# fi

# echo ""
# echo "Results: $PASSED passed, $FAILED failed"
# [ "$FAILED" -eq 0 ]
