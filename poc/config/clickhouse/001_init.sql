CREATE DATABASE IF NOT EXISTS flux;

CREATE TABLE IF NOT EXISTS flux.cell_metrics
(
    ts              DateTime64(3),
    unit            Int8,  -- Changed from UInt8
    stack           String,  -- Simplified from Enum8
    cell            Int8,  -- Changed from UInt8
    voltage_V       Nullable(Float32),
    current_A       Nullable(Float32),
    pressure_mbar   Nullable(Float32),
    temperature_C   Nullable(Float32),
    sensor_id       String,
    status          String,
    quality         Int8
)
ENGINE = MergeTree
PARTITION BY toDate(ts)
ORDER BY (unit, stack, cell, ts);

CREATE MATERIALIZED VIEW IF NOT EXISTS flux.cell_metrics_5m
ENGINE = AggregatingMergeTree()
PARTITION BY toDate(bucket)                      -- ← use bucket
ORDER BY (unit, stack, cell, bucket)
AS
SELECT
    toStartOfInterval(ts, INTERVAL 5 minute) AS bucket,   -- keeps the original ts
    unit, stack, cell,
    avgState(voltage_V)        AS avg_voltage,
    avgState(current_A)        AS avg_current,
    avgState(pressure_mbar)    AS avg_dp,
    avgState(temperature_C)    AS avg_temp
FROM flux.cell_metrics
GROUP BY bucket, unit, stack, cell;
