use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// All mutable state the reporter perists between hook invocations.
/// Stored as JSON in `{project_root}/target/scaffold-reporter/state.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// Unix timestamp recorded at pre-compile.
    pub compile_start: Option<f64>,

    /// Unix timestamp recorded at pre-deploy, keyed by contract name.
    pub deploy_start: BTreeMap<String, f64>,
    /// Unix timestamp recorded at pre-codegen, keyed by contract name.
    pub codegen_start: BTreeMap<String, f64>,
    /// Unix timestamp recorded at pre-dev.
    pub dev_start: Option<f64>,
    /// WASM sizes (bytes) from the *previous* post-compile, keyed by contract name.
    pub prev_wasm_sizes: BTreeMap<String, u64>,
}

fn state_path(project_root: &Path) -> PathBuf {
    project_root
        .join("target")
        .join("scaffold-reporter")
        .join("state.json")
}

pub fn load(project_root: &Path) -> State {
    let path = state_path(project_root);
    std::fs::read_to_string(&path)
        .ok() // Option<String>: None if file missing
        .and_then(|s| serde_json::from_str(&s).ok()) // Option<State>: None if parse fails
        .unwrap_or_default() // fall back to State::default() = all None/empty
}

pub fn save(project_root: &Path, state: &State) -> io::Result<()> {
    let path = state_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state).expect("state serialization failed");
    std::fs::write(&path, json)
}

pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

pub fn elapsed_since(start: f64) -> f64 {
    now() - start
}
