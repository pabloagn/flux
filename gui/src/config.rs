use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{env, fs::File, io::BufReader, path::PathBuf, sync::Arc};

/*──────────────────────────── data structs ───────────────────────────*/

#[derive(Debug, Deserialize, Clone)]
pub struct Geometry {
    pub units: Vec<u8>,
    pub stacks: Vec<char>,
    pub cells_per_stack: u8,
    pub membrane_area_m2: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Topics {
    pub voltage: String,
    pub current: String,
    pub pressure: String,
    pub temp_anolyte: String,
    pub temp_catholyte: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlantLayout {
    pub geometry: Geometry,
    pub topics: Topics,
}

/*──────────────────────────── cached instance ─────────────────────────*/

static CONFIG: OnceCell<Arc<PlantLayout>> = OnceCell::new();

pub fn config() -> Arc<PlantLayout> {
    CONFIG
        .get_or_init(|| Arc::new(load().expect("plant_layout.json not found")))
        .clone()
}

/*──────────────────────────── loader ───────────────────────────*/

fn load() -> Result<PlantLayout> {
    let paths = candidate_paths();

    for p in &paths {
        if p.is_file() {
            let file = File::open(p).with_context(|| format!("opening {}", p.display()))?;
            let rdr = BufReader::new(file);
            return Ok(
                serde_json::from_reader(rdr).with_context(|| format!("parsing {}", p.display()))?
            );
        }
    }

    anyhow::bail!(
        "plant_layout.json not found. Looked in:\n{}",
        paths
            .iter()
            .map(|p| format!("  • {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/*──────────────────────────── search order ─────────────────────────*/

fn candidate_paths() -> Vec<PathBuf> {
    let mut v = Vec::new();

    // 1. explicit env-var
    if let Ok(p) = env::var("FLUX_PLANT_LAYOUT") {
        v.push(PathBuf::from(p));
    }

    // 2. current dir
    v.push(PathBuf::from("plant_layout.json"));

    // 3. workspace-root relative
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // ../   (gui/)
        .map(|p| p.join("config/plant/plant_layout.json"));
    if let Some(p) = repo_root {
        v.push(p);
    }

    // 4. system location
    v.push(PathBuf::from("/etc/flux/plant_layout.json"));

    v
}
