"""Test QuestDB pipeline end-to-end"""

import asyncio
import json
from datetime import datetime, timezone
from typing import List

import pytest
import psycopg
from aiokafka import AIOKafkaProducer


@pytest.fixture
async def kafka_producer():
    """Create Kafka producer for testing"""
    producer = AIOKafkaProducer(bootstrap_servers="localhost:9092", value_serializer=lambda v: json.dumps(v).encode())
    await producer.start()
    yield producer
    await producer.stop()


@pytest.fixture
async def questdb_connection():
    """Create QuestDB connection"""
    conn = await psycopg.AsyncConnection.connect(
        "host=localhost port=8812 dbname=qdb user=flux_operator password=flux_questdb_2024"
    )
    yield conn
    await conn.close()


@pytest.mark.asyncio
async def test_pipeline_flow(kafka_producer, questdb_connection):
    """Test complete flow: Kafka -> QuestDB Writer -> QuestDB"""

    # Generate test data
    test_metrics = []
    base_time = datetime.now(timezone.utc)

    for unit_id in range(1, 4):
        for stack_id in ["A", "B", "C"]:
            for cell_id in range(1, 21):
                metric = {
                    "ts": base_time.isoformat(),
                    "unit_id": unit_id,
                    "stack_id": stack_id,
                    "cell_id": cell_id,
                    "voltage_v": 3.0 + (cell_id * 0.01),
                    "current_a": 5000.0,
                    "temperature_c": 85.0 + (cell_id * 0.1),
                    "pressure_mbar": 150.0,
                    "efficiency_pct": 94.5,
                    "power_kw": 15.0,
                    "sensor_quality": 1.0,
                }
                test_metrics.append(metric)

    # Send to Kafka
    print(f"Sending {len(test_metrics)} test metrics to Kafka...")
    for metric in test_metrics:
        await kafka_producer.send("flux.electrical.realtime", value=metric)
    await kafka_producer.flush()

    # Wait for processing
    print("Waiting for QuestDB writer to process...")
    await asyncio.sleep(5)

    # Query QuestDB
    async with questdb_connection.cursor() as cur:
        await cur.execute(
            """
            SELECT COUNT(*) as count
            FROM cell_metrics
            WHERE ts >= $1
        """,
            [base_time],
        )

        result = await cur.fetchone()
        count = result[0]

    print(f"Found {count} records in QuestDB")
    assert count == len(test_metrics), f"Expected {len(test_metrics)}, got {count}"


@pytest.mark.asyncio
async def test_high_throughput(kafka_producer, questdb_connection):
    """Test high throughput scenario"""

    # Generate 100K messages
    batch_size = 10000
    num_batches = 10

    print(f"Sending {batch_size * num_batches} messages...")
    start_time = asyncio.get_event_loop().time()

    for batch in range(num_batches):
        futures = []
        for i in range(batch_size):
            metric = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "unit_id": (i % 3) + 1,
                "stack_id": chr(65 + (i % 3)),  # A, B, C
                "cell_id": (i % 20) + 1,
                "voltage_v": 3.0 + (i % 100) * 0.001,
                "current_a": 5000.0,
                "temperature_c": 85.0,
                "efficiency_pct": 94.5,
                "power_kw": 15.0,
            }
            future = kafka_producer.send("flux.electrical.realtime", value=metric)
            futures.append(future)

        # Wait for batch to complete
        await asyncio.gather(*futures)
        print(f"Batch {batch + 1}/{num_batches} sent")

    elapsed = asyncio.get_event_loop().time() - start_time
    rate = (batch_size * num_batches) / elapsed
    print(f"Send rate: {rate:.0f} messages/second")

    # Wait for processing
    print("Waiting for processing...")
    await asyncio.sleep(10)

    # Verify in QuestDB
    async with questdb_connection.cursor() as cur:
        await cur.execute("""
            SELECT COUNT(*) as count
            FROM cell_metrics
            WHERE ts > dateadd('m', -1, now())
        """)

        result = await cur.fetchone()
        count = result[0]

    print(f"QuestDB contains {count} recent records")
    assert count >= batch_size * num_batches * 0.95  # Allow 5% loss


if __name__ == "__main__":
    # Run tests
    asyncio.run(test_pipeline_flow(None, None))
