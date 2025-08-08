#!/usr/bin/env python
"""Delete existing GlassFlow pipeline."""

from glassflow_clickhouse_etl import Pipeline

pipeline = Pipeline(url="http://localhost:8080")
try:
    existing = pipeline.get_running_pipeline()
    print(f"Found pipeline: {existing}")
    pipeline.delete()
    print("Pipeline deleted")
except Exception as e:
    print(f"No pipeline to delete: {e}")
