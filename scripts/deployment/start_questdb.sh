#!/usr/bin/env bash
set -e

echo "Starting QuestDB stack..."

# Create data directories
mkdir -p data/questdb
chmod 777 data/questdb # QuestDB needs write access

# Start QuestDB
docker-compose -f infra/docker/docker-compose.questdb.yml up -d questdb

# Wait for QuestDB to be ready
echo "Waiting for QuestDB to be ready..."
for i in {1..30}; do
  if curl -f http://localhost:9000/exec?query=SELECT%201 >/dev/null 2>&1; then
    echo "QuestDB is ready!"
    break
  fi
  echo "Waiting... ($i/30)"
  sleep 2
done

# Initialize schema
echo "Initializing QuestDB schema..."
curl -G http://localhost:9000/exec \
  --data-urlencode "query=$(cat config/schemas/questdb/init.sql)"

# Start writer service
echo "Starting QuestDB writer..."
docker-compose -f infra/docker/docker-compose.questdb.yml up -d questdb-writer

echo "QuestDB stack is running!"
echo "Web Console: http://localhost:9000"
echo "PostgreSQL: localhost:8812"
