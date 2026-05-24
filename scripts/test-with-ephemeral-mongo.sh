#!/usr/bin/env bash
set -euo pipefail

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required but not installed or not in PATH" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required but not installed or not in PATH" >&2
  exit 1
fi

CONTAINER_NAME="credit-exchanger-test-db-$(date +%s)-$RANDOM"
DB_IMAGE="mongo:7"
HOST_PORT=""

cleanup() {
  if [[ -n "${CONTAINER_NAME}" ]]; then
    docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

# Publish to a random host port to avoid collisions with local MongoDB.
docker run -d --name "$CONTAINER_NAME" -p 127.0.0.1::27017 "$DB_IMAGE" >/dev/null

HOST_PORT="$(docker port "$CONTAINER_NAME" 27017/tcp | awk -F: '{print $2}')"
if [[ -z "${HOST_PORT}" ]]; then
  echo "failed to determine mapped MongoDB port" >&2
  exit 1
fi

TEST_MONGODB_URI="mongodb://127.0.0.1:${HOST_PORT}"
export TEST_MONGODB_URI

echo "MongoDB test container: ${CONTAINER_NAME}"
echo "MongoDB URI: ${TEST_MONGODB_URI}"

# Wait for MongoDB to accept commands.
for _ in {1..40}; do
  if docker exec "$CONTAINER_NAME" mongosh --quiet --eval "db.runCommand({ ping: 1 })" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! docker exec "$CONTAINER_NAME" mongosh --quiet --eval "db.runCommand({ ping: 1 })" >/dev/null 2>&1; then
  echo "MongoDB did not become ready in time" >&2
  exit 1
fi

cargo test "$@"
