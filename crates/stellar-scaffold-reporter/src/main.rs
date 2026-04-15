//! Stellar Scaffold Reporter extension.
//!
//! Reports build metrics after each phase of the scaffold lifecycle. Install
//! this extension and add it to your `environments.toml` to see timing,
//! WASM sizes, deployment details, and build-cycle summaries without cluttering
//! the main scaffold output.
//!
//! # Configuration
//!
//! Configure via `[<env>.ext.reporter]` in `environments.toml`:
//!
//! ```toml
//! [development.ext.reporter]
//! mode = "standard"        # "standard" (default) | "minimal"
//! warn_size_kb = 128       # warn if any WASM exceeds this size in KB
//! log_file = "target/scaffold-reporter/build.log"  # optional log file
//! ```
//!
//! ## Modes
//!
//! - `standard` (default): compile timing + WASM sizes, deploy details,
//!   codegen duration, and a post-dev build summary.
//! - `minimal`: post-dev build summary only, plus any WASM size warnings.
//!
//! WASM size warnings (`warn_size_kb`) are emitted regardless of mode.

use crate::report::Reporter;
use clap::{Parser, Subcommand};
use stellar_scaffold_ext_types::{
    CodegenContext, CompileContext, DeployContext, DeployKind, ExtensionManifest, HookName,
    ProjectContext,
};
pub mod report;
pub mod state;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Default, serde::Deserialize)]
struct Config {
    #[serde(default)]
    mode: Mode,
    warn_size_kb: Option<f64>,
    log_file: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Mode {
    /// Only emit the post-dev build summary and WASM size warnings.
    Minimal,
    /// Emit per-contract metrics: compile timing + WASM sizes, deploy details,
    /// and codegen duration, plus the post-dev summary.
    #[default]
    Standard,
}

fn parse_config(config: Option<&serde_json::Value>) -> Config {
    config
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

fn log_path(project_root: &std::path::Path, config: &Config) -> Option<std::path::PathBuf> {
    config.log_file.as_ref().map(|f| project_root.join(f))
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Manifest,
    PreCompile,
    PostCompile,
    PreDeploy,
    PostDeploy,
    PreCodegen,
    PostCodegen,
    PreDev,
    PostDev,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Manifest => cmd_manifest(),
        Command::PreCompile => cmd_pre_compile(),
        Command::PostCompile => cmd_post_compile(),
        Command::PreDeploy => cmd_pre_deploy(),
        Command::PostDeploy => cmd_post_deploy(),
        Command::PreCodegen => cmd_pre_codegen(),
        Command::PostCodegen => cmd_post_codegen(),
        Command::PreDev => cmd_pre_dev(),
        Command::PostDev => cmd_post_dev(),
    }
}

fn cmd_manifest() {
    let manifest = ExtensionManifest {
        name: String::from("reporter"),
        version: String::from(env!("CARGO_PKG_VERSION")),
        hooks: [
            HookName::PreCompile,
            HookName::PostCompile,
            HookName::PreDeploy,
            HookName::PostDeploy,
            HookName::PreCodegen,
            HookName::PostCodegen,
            HookName::PreDev,
            HookName::PostDev,
        ]
        .map(|h| h.as_str().to_string())
        .to_vec(),
    };
    println!("{}", serde_json::json!(manifest));
}

/// Reads a context JSON from stdin and deserializes it.
///
/// Returns `None` on I/O or parse failure after logging a diagnostic to
/// stderr. The reporter is a child process of scaffold; panicking or
/// returning a non-zero exit would surface in scaffold's output as an
/// extension error, so we degrade gracefully instead.
fn read_stdin<T: serde::de::DeserializeOwned>() -> Option<T> {
    use std::io::Read;
    let mut buf = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
        eprintln!("stellar-scaffold-reporter: failed to read stdin: {e}");
        return None;
    }
    match serde_json::from_str(&buf) {
        Ok(ctx) => Some(ctx),
        Err(e) => {
            eprintln!("stellar-scaffold-reporter: failed to parse context JSON: {e}");
            None
        }
    }
}

/// Persist reporter state to disk, logging (but not propagating) I/O errors.
///
/// State loss only corrupts future deltas and timings, so a diagnostic is
/// preferable to failing the hook.
fn save_state(project_root: &std::path::Path, state: &state::State) {
    if let Err(e) = state::save(project_root, state) {
        eprintln!("stellar-scaffold-reporter: failed to save state: {e}");
    }
}

// ---------------------------------------------------------------------------
// Hook handlers
// ---------------------------------------------------------------------------

fn cmd_pre_compile() {
    let Some(ctx) = read_stdin::<CompileContext>() else {
        return;
    };
    let mut state = state::load(&ctx.project_root);
    state.compile_start = Some(state::now());
    save_state(&ctx.project_root, &state);
}

fn cmd_post_compile() {
    let Some(ctx) = read_stdin::<CompileContext>() else {
        return;
    };
    let config = parse_config(ctx.config.as_ref());
    let mut state = state::load(&ctx.project_root);
    let mut reporter = Reporter::new(log_path(&ctx.project_root, &config).as_deref());

    if config.mode == Mode::Standard {
        // Compile duration
        if let Some(start) = state.compile_start {
            let elapsed = state::elapsed_since(start);
            reporter.log(&format!("📋 Compile time: {elapsed:.2}s"));
        }

        // WASM sizes
        reporter.log("📋 WASM sizes:");
        for (name, path) in &ctx.wasm_paths {
            let result = match std::fs::metadata(path) {
                Ok(meta) => {
                    let size = meta.len();
                    // WASM files will be small enough for precision loss to not matter
                    #[allow(clippy::cast_precision_loss)]
                    let size_kb = size as f64 / 1024.0;
                    let delta = state
                        .prev_wasm_sizes
                        .get(name)
                        .map(|&prev| {
                            let diff = i128::from(size) - i128::from(prev);
                            match diff.cmp(&0) {
                                std::cmp::Ordering::Greater => format!(" (▲{diff}B)"),
                                std::cmp::Ordering::Less => {
                                    format!(" (▼{}B)", diff.unsigned_abs())
                                }
                                std::cmp::Ordering::Equal => " (no change)".to_string(),
                            }
                        })
                        .unwrap_or_default();
                    format!("{name}: {size_kb:.1}KB{delta}")
                }
                Err(_) => String::from("(⚠️ WASM not found)"),
            };
            reporter.log(&format!("    • {result}"));
        }
    }

    // WASM size warnings — always emitted regardless of verbosity
    if let Some(warn_kb) = config.warn_size_kb {
        for (name, path) in &ctx.wasm_paths {
            if let Ok(meta) = std::fs::metadata(path) {
                #[allow(clippy::cast_precision_loss)]
                let size_kb = meta.len() as f64 / 1024.0;
                if size_kb > warn_kb {
                    reporter.log(&format!(
                        "⚠️  {name}: WASM size {size_kb:.1}KB exceeds threshold of {warn_kb:.0}KB"
                    ));
                }
            }
        }
    }

    // Update state for next build
    state.prev_wasm_sizes = ctx
        .wasm_paths
        .iter()
        .filter_map(|(name, path)| {
            std::fs::metadata(path)
                .ok()
                .map(|m| (name.clone(), m.len()))
        })
        .collect();
    state.compile_start = None;
    save_state(&ctx.project_root, &state);
}

fn cmd_pre_deploy() {
    let Some(ctx) = read_stdin::<DeployContext>() else {
        return;
    };
    let mut state = state::load(&ctx.compile.project_root);
    state
        .deploy_start
        .insert(ctx.contract_name.clone(), state::now());
    save_state(&ctx.compile.project_root, &state);
}

fn cmd_post_deploy() {
    let Some(ctx) = read_stdin::<DeployContext>() else {
        return;
    };
    let config = parse_config(ctx.compile.config.as_ref());
    let mut state = state::load(&ctx.compile.project_root);
    let mut reporter = Reporter::new(log_path(&ctx.compile.project_root, &config).as_deref());

    if config.mode == Mode::Standard {
        let elapsed = state.deploy_start.remove(&ctx.contract_name).map_or_else(
            || "?".to_string(),
            |start| format!("{:.2}s", state::elapsed_since(start)),
        );

        let contract_id = ctx.contract_id.as_deref().unwrap_or("(unknown)");
        let kind = match ctx.deploy_kind {
            Some(DeployKind::Upgraded) => "upgraded in-place",
            Some(DeployKind::Unchanged) => "unchanged",
            Some(DeployKind::Fresh) | None => "deployed fresh",
            // DeployKind is #[non_exhaustive]; future variants default to this.
            Some(_) => "deployed",
        };

        reporter.log(&format!(
            "📋 Deployed {} ({kind}):\n    id = {}\n    hash = {}\n    duration = {}",
            ctx.contract_name, contract_id, &ctx.wasm_hash, elapsed,
        ));
    } else {
        // Still remove the timer entry so state stays clean
        state.deploy_start.remove(&ctx.contract_name);
    }

    save_state(&ctx.compile.project_root, &state);
}

fn cmd_pre_codegen() {
    let Some(ctx) = read_stdin::<CodegenContext>() else {
        return;
    };
    let mut state = state::load(&ctx.deploy.compile.project_root);
    state
        .codegen_start
        .insert(ctx.deploy.contract_name, state::now());
    save_state(&ctx.deploy.compile.project_root, &state);
}

fn cmd_post_codegen() {
    let Some(ctx) = read_stdin::<CodegenContext>() else {
        return;
    };
    let config = parse_config(ctx.deploy.compile.config.as_ref());
    let mut state = state::load(&ctx.deploy.compile.project_root);
    let mut reporter =
        Reporter::new(log_path(&ctx.deploy.compile.project_root, &config).as_deref());

    if config.mode == Mode::Standard {
        let elapsed = state
            .codegen_start
            .remove(&ctx.deploy.contract_name)
            .map_or_else(
                || "?".to_string(),
                |start| format!("{:.2}s", state::elapsed_since(start)),
            );

        // Sum the sizes of all files in ts_package_dir recursively
        let ts_size_kb = dir_size_kb(&ctx.ts_package_dir);

        reporter.log(&format!(
            "📋 Codegen {}:\n    duration = {}\n    package size = {:.1}KB",
            ctx.deploy.contract_name, elapsed, ts_size_kb,
        ));
    } else {
        state.codegen_start.remove(&ctx.deploy.contract_name);
    }

    save_state(&ctx.deploy.compile.project_root, &state);
}

/// Returns total size of all files under `dir` recursively, in KB.
#[allow(clippy::cast_precision_loss)]
fn dir_size_kb(dir: &std::path::Path) -> f64 {
    fn visit(dir: &std::path::Path, total: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, total);
            } else if let Ok(meta) = path.metadata() {
                *total += meta.len();
            }
        }
    }
    let mut total = 0u64;
    visit(dir, &mut total);
    total as f64 / 1024.0
}

fn cmd_pre_dev() {
    let Some(ctx) = read_stdin::<ProjectContext>() else {
        return;
    };
    let mut state = state::load(&ctx.project_root);
    state.dev_start = Some(state::now());
    save_state(&ctx.project_root, &state);
}

fn cmd_post_dev() {
    let Some(ctx) = read_stdin::<ProjectContext>() else {
        return;
    };
    let config = parse_config(ctx.config.as_ref());
    let mut state = state::load(&ctx.project_root);
    let mut reporter = Reporter::new(log_path(&ctx.project_root, &config).as_deref());

    if let Some(start) = state.dev_start.take() {
        let elapsed = state::elapsed_since(start);

        let (succeeded, failed): (Vec<_>, Vec<_>) =
            ctx.contracts.iter().partition(|c| c.wasm_path.is_some());

        let summary = if failed.is_empty() {
            format!(
                "📋 build cycle complete: {} contract(s) in {elapsed:.2}s",
                succeeded.len()
            )
        } else {
            let failed_names: Vec<&str> = failed.iter().map(|c| c.name.as_str()).collect();
            format!(
                "📋 build cycle complete: {} succeeded, {} failed ({}) in {elapsed:.2}s",
                succeeded.len(),
                failed.len(),
                failed_names.join(", ")
            )
        };

        reporter.log(&summary);
    }

    save_state(&ctx.project_root, &state);
}
