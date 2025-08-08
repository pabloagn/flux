#!/usr/bin/env python3
"""
High-performance Kafka to QuestDB writer
Handles 1M+ events/second with batching and connection pooling
"""

import asyncio
import os
import signal
import sys
from contextlib import asynccontextmanager
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional

import orjson
import psycopg
import structlog
import uvloop
from aiokafka import AIOKafkaConsumer
from prometheus_client import Counter, Histogram, start_http_server
from psycopg import AsyncConnection
from psycopg.pool import AsyncConnectionPool
from psycopg.rows import dict_row
from pydantic import BaseModel, Field
from tenacity import retry, stop_after_attempt, wait_exponential

# Performance optimization
asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())

# Structured logging
logger = structlog.get_logger()

# Metrics
MESSAGES_PROCESSED = Counter("questdb_messages_processed_total", "Total messages processed")
MESSAGES_FAILED = Counter("questdb_messages_failed_total", "Total messages failed")
BATCH_SIZE_HISTOGRAM = Histogram("questdb_batch_size", "Batch sizes")
WRITE_LATENCY = Histogram("questdb_write_latency_seconds", "Write latency")


class CellMetric(BaseModel):
    """Cell measurement data model"""

    ts: datetime
    unit_id: int = Field(ge=1, le=3)
    stack_id: str = Field(pattern=r"^[A-C]$")
    cell_id: int = Field(ge=1, le=20)
    voltage_v: Optional[float] = None
    current_a: Optional[float] = None
    temperature_c: Optional[float] = None
    pressure_mbar: Optional[float] = None
    efficiency_pct: Optional[float] = None
    power_kw: Optional[float] = None
    specific_energy: Optional[float] = None
    membrane_resistance: Optional[float] = None
    sensor_quality: float = Field(default=1.0, ge=0.0, le=1.0)

    class Config:
        json_encoders = {datetime: lambda v: v.isoformat()}


class QuestDBWriter:
    """High-performance QuestDB writer with connection pooling"""

    def __init__(
        self,
        host: str = "localhost",
        port: int = 8812,
        database: str = "qdb",
        user: str = "flux_operator",
        password: str = "flux_questdb_2024",
        pool_size: int = 10,
        batch_size: int = 10000,
        batch_timeout_ms: int = 100,
    ):
        self.host = host
        self.port = port
        self.database = database
        self.user = user
        self.password = password
        self.pool_size = pool_size
        self.batch_size = batch_size
        self.batch_timeout_ms = batch_timeout_ms
        self.pool: Optional[AsyncConnectionPool] = None
        self.batch: List[CellMetric] = []
        self.last_flush = asyncio.get_event_loop().time()

    async def connect(self):
        """Initialize connection pool"""
        conninfo = (
            f"host={self.host} port={self.port} "
            f"dbname={self.database} user={self.user} password={self.password} "
            "sslmode=disable options='-c statement_timeout=10000'"
        )

        self.pool = AsyncConnectionPool(
            conninfo,
            min_size=2,
            max_size=self.pool_size,
            timeout=30,
            max_lifetime=600,
            max_idle=300,
        )
        await self.pool.wait()
        logger.info("QuestDB connection pool initialized", pool_size=self.pool_size)

    async def close(self):
        """Close connection pool"""
        if self.pool:
            await self.pool.close()

    @retry(stop=stop_after_attempt(3), wait=wait_exponential(multiplier=1, min=1, max=10))
    async def write_batch(self, metrics: List[CellMetric]):
        """Write batch to QuestDB using COPY for maximum performance"""
        if not metrics:
            return

        with WRITE_LATENCY.time():
            async with self.pool.connection() as conn:
                async with conn.cursor() as cur:
                    # Prepare data for COPY
                    rows = []
                    for m in metrics:
                        rows.append(
                            (
                                m.ts,
                                m.unit_id,
                                m.stack_id,
                                m.cell_id,
                                m.voltage_v,
                                m.current_a,
                                m.temperature_c,
                                m.pressure_mbar,
                                m.efficiency_pct,
                                m.power_kw,
                                m.specific_energy,
                                m.membrane_resistance,
                                m.sensor_quality,
                            )
                        )

                    # Use COPY for bulk insert (fastest method)
                    async with cur.copy(
                        """COPY cell_metrics (
                            ts, unit_id, stack_id, cell_id, voltage_v, current_a,
                            temperature_c, pressure_mbar, efficiency_pct, power_kw,
                            specific_energy, membrane_resistance, sensor_quality
                        ) FROM STDIN"""
                    ) as copy:
                        for row in rows:
                            await copy.write_row(row)

                    MESSAGES_PROCESSED.inc(len(metrics))
                    BATCH_SIZE_HISTOGRAM.observe(len(metrics))

        logger.debug("Batch written", size=len(metrics))

    async def add_metric(self, metric: CellMetric):
        """Add metric to batch and flush if needed"""
        self.batch.append(metric)

        # Flush if batch is full or timeout reached
        current_time = asyncio.get_event_loop().time()
        if len(self.batch) >= self.batch_size or (current_time - self.last_flush) * 1000 >= self.batch_timeout_ms:
            await self.flush()

    async def flush(self):
        """Flush current batch to QuestDB"""
        if not self.batch:
            return

        batch_to_write = self.batch.copy()
        self.batch.clear()
        self.last_flush = asyncio.get_event_loop().time()

        try:
            await self.write_batch(batch_to_write)
        except Exception as e:
            logger.error("Failed to write batch", error=str(e), batch_size=len(batch_to_write))
            MESSAGES_FAILED.inc(len(batch_to_write))
            # In production, implement dead letter queue here
            raise


class KafkaToQuestDB:
    """Kafka consumer that writes to QuestDB"""

    def __init__(
        self,
        kafka_brokers: str = "localhost:9092",
        kafka_topics: List[str] = None,
        kafka_group: str = "questdb-writer",
        questdb_config: Dict[str, Any] = None,
    ):
        self.kafka_brokers = kafka_brokers
        self.kafka_topics = kafka_topics or [
            "flux.electrical.realtime",
            "flux.process.temperatures",
            "flux.process.pressures",
        ]
        self.kafka_group = kafka_group
        self.consumer: Optional[AIOKafkaConsumer] = None
        self.writer = QuestDBWriter(**(questdb_config or {}))
        self.running = False

    async def start(self):
        """Start consumer and writer"""
        await self.writer.connect()

        self.consumer = AIOKafkaConsumer(
            *self.kafka_topics,
            bootstrap_servers=self.kafka_brokers,
            group_id=self.kafka_group,
            value_deserializer=lambda m: orjson.loads(m),
            enable_auto_commit=False,
            auto_offset_reset="latest",
            max_poll_records=10000,
            fetch_max_bytes=52428800,  # 50MB
            session_timeout_ms=30000,
            heartbeat_interval_ms=3000,
        )

        await self.consumer.start()
        self.running = True
        logger.info("Kafka consumer started", topics=self.kafka_topics)

    async def stop(self):
        """Stop consumer and writer"""
        self.running = False

        if self.consumer:
            await self.consumer.stop()

        await self.writer.flush()
        await self.writer.close()
        logger.info("Kafka consumer stopped")

    async def process_messages(self):
        """Main message processing loop"""
        flush_task = asyncio.create_task(self.periodic_flush())

        try:
            async for msg in self.consumer:
                if not self.running:
                    break

                try:
                    # Parse message
                    data = msg.value

                    # Add timestamp if missing
                    if "ts" not in data:
                        data["ts"] = datetime.now(timezone.utc)
                    elif isinstance(data["ts"], str):
                        data["ts"] = datetime.fromisoformat(data["ts"])

                    # Validate and add to batch
                    metric = CellMetric(**data)
                    await self.writer.add_metric(metric)

                    # Commit offset periodically (every 1000 messages)
                    if msg.offset % 1000 == 0:
                        await self.consumer.commit()

                except Exception as e:
                    logger.error(
                        "Failed to process message",
                        error=str(e),
                        topic=msg.topic,
                        partition=msg.partition,
                        offset=msg.offset,
                    )
                    MESSAGES_FAILED.inc()

        finally:
            flush_task.cancel()
            await self.writer.flush()

    async def periodic_flush(self):
        """Periodic flush task"""
        while self.running:
            await asyncio.sleep(0.1)  # 100ms
            await self.writer.flush()


async def main():
    """Main entry point"""
    # Configuration from environment
    kafka_brokers = os.getenv("KAFKA_BROKERS", "localhost:9092")
    questdb_host = os.getenv("QUESTDB_HOST", "localhost")
    questdb_port = int(os.getenv("QUESTDB_PORT", "8812"))

    # Start metrics server
    start_http_server(8000)

    # Create pipeline
    pipeline = KafkaToQuestDB(
        kafka_brokers=kafka_brokers,
        questdb_config={
            "host": questdb_host,
            "port": questdb_port,
            "batch_size": 10000,
            "batch_timeout_ms": 100,
            "pool_size": 10,
        },
    )

    # Handle signals
    loop = asyncio.get_event_loop()
    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, lambda: asyncio.create_task(shutdown(pipeline)))

    try:
        await pipeline.start()
        await pipeline.process_messages()
    except Exception as e:
        logger.error("Pipeline failed", error=str(e))
        sys.exit(1)
    finally:
        await pipeline.stop()


async def shutdown(pipeline):
    """Graceful shutdown"""
    logger.info("Shutting down...")
    pipeline.running = False


if __name__ == "__main__":
    asyncio.run(main())
