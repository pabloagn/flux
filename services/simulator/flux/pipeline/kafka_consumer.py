"""Kafka consumer for verification."""

from kafka import KafkaConsumer
import json


def consume_messages(
    topic: str, bootstrap_servers: str = "localhost:9092", max_messages: int = 10
):
    """Consume and print messages from a topic."""

    consumer = KafkaConsumer(
        topic,
        bootstrap_servers=[bootstrap_servers],
        auto_offset_reset="earliest",
        value_deserializer=lambda m: json.loads(m.decode("utf-8")),
        consumer_timeout_ms=5000,  # Stop after 5 seconds of no messages
    )

    count = 0
    for message in consumer:
        print(f"Offset {message.offset}: {message.value}")
        count += 1
        if count >= max_messages:
            break

    consumer.close()
    return count
