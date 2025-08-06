#!/usr/bin/env bash
echo "Initializing ClickHouse schema..."
docker exec -i poc-clickhouse-1 clickhouse-client < config/clickhouse/001_init.sql
echo "Schema created"
