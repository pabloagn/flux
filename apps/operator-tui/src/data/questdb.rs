use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Config, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_postgres::{NoTls, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMetric {
    pub ts: DateTime<Utc>,
    pub unit_id: i32,
    pub stack_id: String,
    pub cell_id: i32,
    pub voltage_v: Option<f64>,
    pub current_a: Option<f64>,
    pub temperature_c: Option<f64>,
    pub pressure_mbar: Option<f64>,
    pub efficiency_pct: Option<f64>,
    pub power_kw: Option<f64>,
    pub specific_energy: Option<f64>,
    pub membrane_resistance: Option<f64>,
    pub sensor_quality: Option<f64>,
}

impl From<Row> for CellMetric {
    fn from(row: Row) -> Self {
        Self {
            ts: row.get("ts"),
            unit_id: row.get("unit_id"),
            stack_id: row.get("stack_id"),
            cell_id: row.get("cell_id"),
            voltage_v: row.get("voltage_v"),
            current_a: row.get("current_a"),
            temperature_c: row.get("temperature_c"),
            pressure_mbar: row.get("pressure_mbar"),
            efficiency_pct: row.get("efficiency_pct"),
            power_kw: row.get("power_kw"),
            specific_energy: row.get("specific_energy"),
            membrane_resistance: row.get("membrane_resistance"),
            sensor_quality: row.get("sensor_quality"),
        }
    }
}

pub struct QuestDBClient {
    pool: Pool,
}

impl QuestDBClient {
    pub async fn new(host: &str, port: u16, user: &str, password: &str) -> Result<Self> {
        let mut config = Config::new();
        config.host = Some(host.to_string());
        config.port = Some(port);
        config.user = Some(user.to_string());
        config.password = Some(password.to_string());
        config.dbname = Some("qdb".to_string());
        config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        config.pool = Some(deadpool_postgres::PoolConfig {
            max_size: 16,
            timeouts: deadpool_postgres::Timeouts {
                wait: Some(Duration::from_secs(5)),
                create: Some(Duration::from_secs(5)),
                recycle: Some(Duration::from_secs(5)),
            },
            ..Default::default()
        });

        let pool = config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .context("Failed to create QuestDB connection pool")?;

        Ok(Self { pool })
    }

    /// Get latest metrics for all cells
    pub async fn get_latest_metrics(&self) -> Result<Vec<CellMetric>> {
        let client = self.pool.get().await?;

        let query = r#"
            SELECT 
                ts, unit_id, stack_id, cell_id,
                voltage_v, current_a, temperature_c,
                pressure_mbar, efficiency_pct, power_kw,
                specific_energy, membrane_resistance, sensor_quality
            FROM cell_metrics
            WHERE ts > dateadd('s', -5, now())
            LATEST ON ts PARTITION BY unit_id, stack_id, cell_id
        "#;

        let rows = client.query(query, &[]).await?;
        Ok(rows.into_iter().map(CellMetric::from).collect())
    }

    // Other functions (get_cell_history, get_unit_metrics, etc.) would also need their queries updated
    // if we want them to return the new fields. For now, we focus on the main TUI display.
}

// Stubs for other structs to allow compilation
#[derive(Debug, Serialize, Deserialize)]
pub struct UnitMetrics {
    // ...
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AlarmEvent {
    // ...
}
