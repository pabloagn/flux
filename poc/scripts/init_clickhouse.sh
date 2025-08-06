#!/usr/bin/env bash
set -euo pipefail

echo "Initializing ClickHouse schema..."

ROOT_DIR="$(git rev-parse --show-toplevel)"

SQL_FILE="$ROOT_DIR/poc/config/clickhouse/001_init.sql"

docker exec -i poc-clickhouse-1 clickhouse-client <"$SQL_FILE"

echo "Schema created"
