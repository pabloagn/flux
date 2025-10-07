import asyncio
import os
import signal
import sys
from datetime import datetime, timezone
from typing import List

import orjson
import structlog
import uvloop
from aiokafka import AIOKafkaConsumer
from aiokafka.errors import KafkaConnectionError
from prometheus_client import Counter, Histogram, start_http_server
from tenacity import retry, stop_after_attempt, wait_exponential

from data_pipeline.questdb_writer import CellMetric, QuestDBWriter

asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
logger = structlog.get_logger(__name__)

MESSAGES_PROCESSED = Counter("questdb_messages_processed_total", "Total messages processed")
MESSAGES_FAILED = Counter("questdb_messages_failed_total", "Total messages failed to process")
BATCH_WRITE_SUCCESS = Counter("questdb_batch_write_success_total", "Batches successfully written")
BATCH_WRITE_FAILED = Counter("questdb_batch_write_failed_total", "Batches failed to write")
BATCH_SIZE_HISTOGRAM = Histogram("questdb_batch_size", "Batch sizes")
WRITE_LATENCY = Histogram("questdb_write_latency_seconds", "Write latency")


class KafkaToQuestDB:
    def __init__(
        self,
        kafka_brokers: str,
        kafka_topics: List[str],
        kafka_group: str,
        writer: QuestDBWriter,
        batch_size: int,
        batch_timeout_ms: int,
    ):
        self.kafka_brokers = kafka_brokers
        self.kafka_topics = kafka_topics
        self.kafka_group = kafka_group
        self.writer = writer
        self.batch_size = batch_size
        self.batch_timeout_ms = batch_timeout_ms
        self.batch: List[CellMetric] = []
        self.last_flush = asyncio.get_event_loop().time()
        self.consumer: AIOKafkaConsumer | None = None
        self.running = False

    @retry(
        wait=wait_exponential(multiplier=1, min=2, max=10),
        stop=stop_after_attempt(5),
        retry_error_callback=lambda _: logger.critical("Failed to connect to Kafka after multiple retries."),
    )
    async def start(self):
        logger.info("Attempting to connect to QuestDB...")
        await self.writer.open()
        logger.info("QuestDB connection pool opened.")

        logger.info("Attempting to connect to Kafka...", brokers=self.kafka_brokers)
        self.consumer = AIOKafkaConsumer(
            *self.kafka_topics,
            bootstrap_servers=self.kafka_brokers,
            group_id=self.kafka_group,
            value_deserializer=lambda m: orjson.loads(m),
            enable_auto_commit=False,
            auto_offset_reset="latest",
            max_poll_records=10000,
        )
        await self.consumer.start()
        self.running = True
        logger.info("Kafka consumer started", topics=self.kafka_topics)

    async def stop(self):
        self.running = False
        if self.consumer:
            await self.consumer.stop()
        await self.flush()
        await self.writer.close()
        logger.info("Kafka consumer stopped gracefully.")

    async def _flush_internal(self):
        if not self.batch:
            return
        batch_to_write = self.batch
        self.batch = []
        self.last_flush = asyncio.get_event_loop().time()

        try:
            with WRITE_LATENCY.time():
                await self.writer.write_batch(batch_to_write)
            MESSAGES_PROCESSED.inc(len(batch_to_write))
            BATCH_SIZE_HISTOGRAM.observe(len(batch_to_write))
            BATCH_WRITE_SUCCESS.inc()
            logger.debug("Batch written successfully", size=len(batch_to_write))
        except Exception as e:
            BATCH_WRITE_FAILED.inc()
            logger.error("Failed to write batch to QuestDB", error=str(e), batch_size=len(batch_to_write))

    async def flush(self):
        await self._flush_internal()

    async def add_metric_and_flush_if_needed(self, metric: CellMetric):
        self.batch.append(metric)
        now = asyncio.get_event_loop().time()
        if len(self.batch) >= self.batch_size or (now - self.last_flush) * 1000 >= self.batch_timeout_ms:
            await self._flush_internal()
            if self.consumer:
                await self.consumer.commit()

    async def run(self):
        flush_task = asyncio.create_task(self.periodic_flush())
        try:
            async for msg in self.consumer:
                if not self.running:
                    break
                try:
                    data = msg.value
                    if "ts" not in data:
                        data["ts"] = datetime.now(timezone.utc)
                    metric = CellMetric(**data)
                    await self.add_metric_and_flush_if_needed(metric)
                except Exception as e:
                    MESSAGES_FAILED.inc()
                    logger.error("Failed to parse or process message", error=str(e), topic=msg.topic, offset=msg.offset)
        finally:
            flush_task.cancel()
            await self.flush()
            if self.consumer:
                await self.consumer.commit()

    async def periodic_flush(self):
        while self.running:
            await asyncio.sleep(self.batch_timeout_ms / 1000)
            await self.flush()


async def shutdown(sig, loop, pipeline: KafkaToQuestDB):
    logger.info(f"Received exit signal {sig.name}...")
    await pipeline.stop()
    tasks = [t for t in asyncio.all_tasks() if t is not asyncio.current_task()]
    for task in tasks:
        task.cancel()
    await asyncio.gather(*tasks, return_exceptions=True)
    loop.stop()


async def async_main():
    kafka_brokers = os.environ["KAFKA_BROKERS"]
    questdb_host = os.environ["QUESTDB_HOST"]
    questdb_port = int(os.environ["QUESTDB_PORT"])
    questdb_user = os.environ["QUESTDB_USER"]
    questdb_password = os.environ["QUESTDB_PASSWORD"]
    questdb_db = os.getenv("QUESTDB_DB", "qdb")
    kafka_topics_str = os.environ.get("KAFKA_TOPICS", "flux.cell.metrics.v1")
    kafka_topics = [topic.strip() for topic in kafka_topics_str.split(",")]

    start_http_server(8000)

    writer = QuestDBWriter(
        host=questdb_host,
        port=questdb_port,
        database=questdb_db,
        user=questdb_user,
        password=questdb_password,
    )

    pipeline = KafkaToQuestDB(
        kafka_brokers=kafka_brokers,
        kafka_topics=kafka_topics,
        kafka_group="questdb-writer-group",
        writer=writer,
        batch_size=10000,
        batch_timeout_ms=100,
    )

    loop = asyncio.get_event_loop()
    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, lambda s=sig: asyncio.create_task(shutdown(s, loop, pipeline)))

    try:
        await pipeline.start()
        await pipeline.run()
    except KafkaConnectionError as e:
        logger.critical("Could not connect to Kafka", error=str(e))
        sys.exit(1)
    except Exception as e:
        logger.critical("Pipeline failed with unhandled exception", error=str(e))
        sys.exit(1)


def main():
    structlog.configure(
        processors=[
            structlog.contextvars.merge_contextvars,
            structlog.processors.add_log_level,
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.JSONRenderer(),
        ]
    )
    try:
        asyncio.run(async_main())
    except KeyboardInterrupt:
        logger.info("Shutdown requested by user.")


if __name__ == "__main__":
    main()
