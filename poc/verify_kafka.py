"""Verify Kafka messages are being received."""

from flux.pipeline.kafka_consumer import consume_messages

if __name__ == "__main__":
    print("Reading messages from 'cell_voltage' topic...")
    count = consume_messages("cell_voltage", max_messages=5)
    print(f"\nRead {count} messages")
