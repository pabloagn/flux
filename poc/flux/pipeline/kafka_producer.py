"""Kafka producer for sensor data."""

import json
from kafka import KafkaProducer
from typing import Dict, Any


class SensorDataProducer:
    """Produces sensor data to Kafka."""

    def __init__(self, bootstrap_servers: str = "localhost:9092"):
        """Initialize Kafka producer."""
        self.producer = KafkaProducer(
            bootstrap_servers=[bootstrap_servers],
            value_serializer=lambda v: json.dumps(v).encode("utf-8"),
            key_serializer=lambda k: k.encode("utf-8") if k else None,
        )

    def send(self, topic: str, data: Dict[str, Any], key: str = ""):
        """Send data to Kafka topic."""
        future = self.producer.send(topic, value=data, key=key)
        return future

    def flush(self):
        """Flush all pending messages."""
        self.producer.flush()

    def close(self):
        """Close the producer."""
        self.producer.close()
