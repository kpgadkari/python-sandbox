#!/usr/bin/env bash
# Deploy the full Python Sandbox stack (MariaDB, backend, frontend).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is not installed or not on PATH" >&2
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "error: docker compose plugin is not available" >&2
  exit 1
fi

ENV_FILE="${SANDBOX_ENV_FILE:-.env}"
EXAMPLE="${SANDBOX_ENV_EXAMPLE:-.env.example}"

if [[ ! -f "$ENV_FILE" ]]; then
  if [[ ! -f "$EXAMPLE" ]]; then
    echo "error: missing $ENV_FILE and $EXAMPLE" >&2
    exit 1
  fi
  cp "$EXAMPLE" "$ENV_FILE"
  echo "Created $ENV_FILE from $EXAMPLE."
  echo "Edit passwords in $ENV_FILE before exposing this on your LAN."
fi

# shellcheck disable=SC1090
set -a
source "$ENV_FILE"
set +a

DATA_PATH="${SANDBOX_DATA_PATH:-./data}"
mkdir -p "$DATA_PATH"

echo "Data directory: $DATA_PATH"
echo "HTTP port: ${SANDBOX_HTTP_PORT:-8090}"
echo "Building and starting containers..."
docker compose -f compose.yaml --env-file "$ENV_FILE" up -d --build

HOST="${SANDBOX_PUBLIC_HOST:-}"
if [[ -z "$HOST" ]]; then
  HOST="$(hostname -I 2>/dev/null | awk '{print $1}')"
fi
if [[ -z "$HOST" ]]; then
  HOST="localhost"
fi

PORT="${SANDBOX_HTTP_PORT:-8090}"
echo ""
echo "Python Sandbox is running."
echo "  URL:  http://${HOST}:${PORT}/"
echo "  User: ${SANDBOX_USERNAME:-parent}"
echo ""
echo "Logs:  docker compose -f compose.yaml logs -f"
echo "Stop:  docker compose -f compose.yaml down"
