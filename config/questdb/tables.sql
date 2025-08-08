-- Additional configuration and optimizations

-- Set table parameters for high-frequency data
ALTER TABLE cell_metrics SET PARAM maxUncommittedRows = 1000000;
ALTER TABLE unit_metrics SET PARAM maxUncommittedRows = 100000;

-- Create materialized views for common queries (QuestDB 7.4+)
-- TODO: For now, we'll handle aggregations in the application layer
