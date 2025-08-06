#!/usr/bin/env bash
#
# create_topics.sh  –  create the Kafka topics needed by Flux POC
#
#   • Finds the running Kafka container automatically
#   • Uses underscore topic names (no dots) to stay NATS-safe
#

set -euo pipefail

# ──────────────────────────────────────────────────────────────
# 1. Locate the Kafka container started by docker-compose
# ──────────────────────────────────────────────────────────────
KAFKA_CONTAINER=$(docker ps \
  --filter "ancestor=confluentinc/cp-kafka:7.5.5" \
  --format '{{.Names}}' | head -n 1)

if [[ -z "$KAFKA_CONTAINER" ]]; then
  echo "ERROR: No running cp-kafka container found. Start the stack first." >&2
  exit 1
fi

echo "Creating Kafka topics in container: $KAFKA_CONTAINER"
echo

# ──────────────────────────────────────────────────────────────
# 2. Topics to create (no dots!)
# ──────────────────────────────────────────────────────────────
TOPICS=(
  flux_electrical_realtime
  flux_process_pressures
  flux_process_temperatures
)

for topic in "${TOPICS[@]}"; do
  docker exec "$KAFKA_CONTAINER" kafka-topics \
    --create --if-not-exists \
    --topic "$topic" \
    --bootstrap-server localhost:9092 \
    --partitions 3 \
    --replication-factor 1
done

echo
echo "Current topic list:"
docker exec "$KAFKA_CONTAINER" kafka-topics --list --bootstrap-server localhost:9092
