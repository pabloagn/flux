use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{collections::HashMap, env, fs::File, io::BufReader, path::PathBuf, sync::Arc};

// --- Master Config Struct ---
#[derive(Debug, Deserialize, Clone)]
pub struct FluxConfig {
    pub plant: PlantConfig,
    pub financials: FinancialsConfig,
    pub limits: OpsLimitsConfig,
    pub materials: MaterialsConfig,
    pub defaults: DefaultsConfig,
}

// --- Individual Config File Structs ---
#[derive(Debug, Deserialize, Clone)]
pub struct PlantConfig {
    pub geometry: Geometry,
    pub topics: Topics,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Geometry {
    pub units: Vec<u8>,
    pub stacks: Vec<char>,
    pub cells_per_stack: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Topics {
    pub voltage: String,
    pub current: String,
    pub pressure: String,
    pub temp_anolyte: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FinancialsConfig {
    pub electricity_price_per_mwh: f64,
    pub chlorine_price_per_mt: f64,
    pub naoh_price_per_mt: f64,
    pub hydrogen_price_per_mt: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpsLimitsConfig {
    pub warning: Thresholds,
    pub critical: Thresholds,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Thresholds {
    pub voltage_v: f64,
    pub temperature_c: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MaterialsConfig {
    #[serde(flatten)]
    pub properties: HashMap<String, MaterialProperties>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MaterialProperties {
    pub molecular_weight_g_mol: f64,
    pub density_kg_m3: f64,
    pub boiling_point_c: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DefaultsConfig {
    pub cell_state: DefaultCellState,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DefaultCellState {
    pub voltage_v: f64,
    pub current_a: f64,
    pub temperature_c: f64,
    pub pressure_mbar: f64,
    pub efficiency_percent: f64,
}

// --- Cached Instance ---
static CONFIG: OnceCell<Arc<FluxConfig>> = OnceCell::new();

pub fn config() -> Arc<FluxConfig> {
    CONFIG
        .get_or_init(|| Arc::new(load().expect("Failed to load configuration")))
        .clone()
}

// --- Loader Logic ---

/// Generic function to load a JSON file into a given struct type.
fn load_json<T: for<'de> Deserialize<'de>>(file_name: &str) -> Result<T> {
    let path = find_config_file(file_name)
        .with_context(|| format!("Could not find config file '{}'", file_name))?;
    let file = File::open(&path).with_context(|| format!("Opening {}", path.display()))?;
    let rdr = BufReader::new(file);
    serde_json::from_reader(rdr).with_context(|| format!("Parsing {}", path.display()))
}

/// Main loader that assembles the master config from individual files.
fn load() -> Result<FluxConfig> {
    Ok(FluxConfig {
        plant: load_json("plant.json")?,
        financials: load_json("financials.json")?,
        limits: load_json("ops_limits.json")?,
        materials: load_json("materials.json")?,
        defaults: load_json("defaults.json")?,
    })
}

/// Finds a config file by searching in standard locations.
/// TODO: Improve this handling
fn find_config_file(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    // The logic here is to build a list of potential paths where the `config/plant`
    // directory might be, then check them in order.

    // 1. Relative to current working directory (e.g., running `just gui-run` from workspace root)
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("config/plant").join(name));
    }

    // 2. Relative to the crate's manifest (handles `cargo run` from within the `gui` directory)
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let crate_path = PathBuf::from(manifest_dir);
        // Go up one level to the workspace root
        if let Some(workspace_root) = crate_path.parent() {
            candidates.push(workspace_root.join("config/plant").join(name));
        }
    }

    // 3. Relative to the compiled executable's location (for release builds)
    if let Ok(mut exe_path) = env::current_exe() {
        // We expect the binary to be in something like `.../target/release/`
        // So we go up multiple levels to find the project root.
        for _ in 0..4 {
            if exe_path.pop() {
                candidates.push(exe_path.join("config/plant").join(name));
            }
        }
    }

    // 4. Standard system-wide location for a deployed application
    candidates.push(PathBuf::from("/etc/flux/config/plant").join(name));

    // Find the first path in our list that actually exists and is a file.
    candidates.into_iter().find(|p| p.is_file())
}
