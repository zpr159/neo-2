#!/usr/bin/env bash
# Neo Docker entrypoint

set -euo pipefail

NEO_ENV="${NEO_ENV:-production}"
NEO_CONFIG="/etc/neo/config/${NEO_ENV}.toml"

echo "Neo AGI OS starting (env=${NEO_ENV})"

exec "$@"
