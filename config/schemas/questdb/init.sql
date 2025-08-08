-- QuestDB Schema for FLUX Real-Time Data

-- Cell-level measurements (high frequency)
CREATE TABLE IF NOT EXISTS cell_metrics (
    ts TIMESTAMP,
    unit_id INT,
    stack_id SYMBOL,
    cell_id INT,
    voltage_v DOUBLE,
    current_a DOUBLE,
    temperature_c DOUBLE,
    pressure_mbar DOUBLE,
    efficiency_pct DOUBLE,
    power_kw DOUBLE,
    specific_energy DOUBLE,
    membrane_resistance DOUBLE,
    sensor_quality DOUBLE
) TIMESTAMP(ts) PARTITION BY DAY WAL DEDUP UPSERT KEYS(ts, unit_id, stack_id, cell_id);

-- Electrolyzer unit aggregates
CREATE TABLE IF NOT EXISTS unit_metrics (
    ts TIMESTAMP,
    unit_id INT,
    total_power_mw DOUBLE,
    avg_voltage DOUBLE,
    avg_current DOUBLE,
    avg_temperature DOUBLE,
    avg_efficiency DOUBLE,
    production_cl2_kg_h DOUBLE,
    production_naoh_kg_h DOUBLE,
    production_h2_kg_h DOUBLE,
    brine_flow_m3_h DOUBLE,
    brine_concentration_g_l DOUBLE
) TIMESTAMP(ts) PARTITION BY DAY WAL DEDUP UPSERT KEYS(ts, unit_id);

-- Control commands audit trail
CREATE TABLE IF NOT EXISTS control_commands (
    ts TIMESTAMP,
    command_id SYMBOL,
    operator_id SYMBOL,
    target_type SYMBOL,
    target_id STRING,
    command_type SYMBOL,
    old_value DOUBLE,
    new_value DOUBLE,
    signature STRING,
    status SYMBOL,
    execution_time_ms LONG
) TIMESTAMP(ts) PARTITION BY DAY;

-- Alarm events
CREATE TABLE IF NOT EXISTS alarm_events (
    ts TIMESTAMP,
    alarm_id SYMBOL,
    unit_id INT,
    stack_id SYMBOL,
    cell_id INT,
    severity SYMBOL,
    alarm_type SYMBOL,
    value DOUBLE,
    threshold DOUBLE,
    message STRING,
    acknowledged BOOLEAN,
    ack_by SYMBOL,
    ack_ts TIMESTAMP
) TIMESTAMP(ts) PARTITION BY DAY;

-- Plant state snapshot (for recovery)
CREATE TABLE IF NOT EXISTS plant_state (
    ts TIMESTAMP,
    state_json STRING,
    checksum STRING
) TIMESTAMP(ts) PARTITION BY HOUR;

-- Create indexes for common queries
-- QuestDB creates indexes automatically for SYMBOL columns
