use clickhouse::{Client, Row};
use clickhouse::error::Result;
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Row, Deserialize)]
pub struct CellMetrics {
    pub timestamp: DateTime<Utc>,
    pub unit: u8,
    pub stack: String,
    pub cell: u8,
    pub voltage: f64,
    pub efficiency: f64,
    pub specific_energy: f64,
}

pub async fn query_cell_history(
    client: &Client,
    cell_key: &str,
    hours: u32,
) -> Result<Vec<CellMetrics>> {
    let query = format!(
        "SELECT * FROM cell_metrics 
         WHERE cell_key = '{}' 
         AND timestamp > now() - INTERVAL {} HOUR
         ORDER BY timestamp",
        cell_key, hours
    );
    client.query(&query).fetch_all().await
}
