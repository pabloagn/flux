#!/usr/bin/env python
"""Create the GlassFlow → ClickHouse pipeline for Flux (GF ≥ 0.13)."""

import json
from glassflow_clickhouse_etl import Pipeline
from glassflow_clickhouse_etl.models import (
    PipelineConfig,
    SourceConfig,
    KafkaConnectionParams,
    TopicConfig,
    Schema,
    SchemaField,
    SinkConfig,
    TableMapping,
)

# ───────────────────────── Build config ─────────────────────────
config = PipelineConfig(
    pipeline_id="flux-metrics",
    source=SourceConfig(
        type="kafka",
        connection_params=KafkaConnectionParams(
            brokers=["kafka:9093"],
            protocol="PLAINTEXT",
            skip_auth=True,
        ),
        topics=[
            TopicConfig(
                name="flux_electrical_realtime",
                stream_name="flux_electrical_realtime",  # dots illegal in NATS
                initial_offset="earliest",
                schema=Schema(
                    type="json",
                    fields=[
                        SchemaField(name="ts", type="string"),
                        SchemaField(name="unit", type="int32"),
                        SchemaField(name="stack", type="string"),
                        SchemaField(name="cell", type="int32"),
                        SchemaField(name="sensor_id", type="string"),
                        SchemaField(name="status", type="string"),
                        SchemaField(name="quality", type="int32"),
                        SchemaField(name="voltage_V", type="float32"),
                        SchemaField(name="current_A", type="float32"),
                    ],
                ),
            )
        ],
    ),
    sink=SinkConfig(
        type="clickhouse",
        host="clickhouse",
        port="9000",
        database="flux",
        username="default",
        password="",
        table="cell_metrics",
        secure=False,
        max_batch_size=1000,
        max_delay_time="5s",
        table_mapping=[
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="ts",
                column_name="ts",
                column_type="DateTime64",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="unit",
                column_name="unit",
                column_type="Int32",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="stack",
                column_name="stack",
                column_type="String",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="cell",
                column_name="cell",
                column_type="Int32",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="voltage_V",
                column_name="voltage_V",
                column_type="Float32",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="current_A",
                column_name="current_A",
                column_type="Float32",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="sensor_id",
                column_name="sensor_id",
                column_type="String",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="status",
                column_name="status",
                column_type="String",
            ),
            TableMapping(
                source_id="flux_electrical_realtime",
                field_name="quality",
                column_name="quality",
                column_type="Int32",
            ),
        ],
    ),
)

# ───────────────────────── Post config ─────────────────────────
print("Sending config:")
print(json.dumps(config.model_dump(mode="json"), indent=2))

pipeline = Pipeline(config=config, url="http://localhost:8080")
pipeline.create()

print("GlassFlow pipeline created successfully!")
