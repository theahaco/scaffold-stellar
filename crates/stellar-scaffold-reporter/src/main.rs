use crate::report::Reporter;
use clap::{Parser, Subcommand};
use stellar_scaffold_ext_types::{
    CodegenContext, CompileContext, DeployContext, ExtensionManifest, HookName, ProjectContext,
};
pub mod report;
pub mod state;

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

fn log_path(_project_root: &std::path::Path) -> Option<std::path::PathBuf> {
    // TODO load config and join log file path to project root
    None
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

fn read_stdin<T: serde::de::DeserializeOwned>() -> T {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .expect("failed to read stdin");
    serde_json::from_str(&buf).expect("failed to parse context JSON from stdin")
}

fn cmd_pre_compile() {
    let ctx = read_stdin::<CompileContext>();
    let mut state = state::load(&ctx.project_root);
    state.compile_start = Some(state::now());
    state::save(&ctx.project_root, &state);
}

fn cmd_post_compile() {
    let ctx = read_stdin::<CompileContext>();
    let mut state = state::load(&ctx.project_root);
    let mut reporter = Reporter::new(log_path(&ctx.project_root).as_deref());

    // Compile duration
    if let Some(start) = state.compile_start {
        let elapsed = state::elapsed_since(start);
        reporter.log(&format!("📋 Compile time: {elapsed:.2}s"));
    }

    // WASM size
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
                            std::cmp::Ordering::Less => format!(" (▼{}B)", diff.unsigned_abs()),
                            std::cmp::Ordering::Equal => " (no change)".to_string(),
                        }
                    })
                    .unwrap_or_default();
                format!("{name}: {size_kb:.1}KB{delta}")
            }
            Err(_) => String::from("(⚠️ WASM not found)"),
        };
        reporter.log(&format!("    • {name}: {result}"));
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
    state::save(&ctx.project_root, &state);
}

fn cmd_pre_deploy() {
    let ctx = read_stdin::<DeployContext>();
    let mut reporter = Reporter::new(log_path(&ctx.compile.project_root).as_deref());
    reporter.log("📋 pre deploy hook");
    let mut state = state::load(&ctx.compile.project_root);
    state
        .deploy_start
        .insert(ctx.contract_name.clone(), state::now());
    state::save(&ctx.compile.project_root, &state);
}

fn cmd_post_deploy() {
    let ctx = read_stdin::<DeployContext>();
    let mut state = state::load(&ctx.compile.project_root);
    let mut reporter = Reporter::new(log_path(&ctx.compile.project_root).as_deref());
    reporter.log("📋 post deploy hook");

    let elapsed = state.deploy_start.remove(&ctx.contract_name).map_or_else(
        || "?".to_string(),
        |start| format!("{:.2}s", state::elapsed_since(start)),
    );

    let contract_id = ctx.contract_id.as_deref().unwrap_or("(unknown)");

    reporter.log(&format!(
        "📋 Deployed {}:\n    id = {}\n    hash = {}\n    duration = {}",
        ctx.contract_name, &contract_id, &ctx.wasm_hash, elapsed,
    ));

    state::save(&ctx.compile.project_root, &state);
}

fn cmd_pre_codegen() {
    let ctx = read_stdin::<CodegenContext>();
    let mut reporter = Reporter::new(log_path(&ctx.deploy.compile.project_root).as_deref());
    reporter.log("📋 pre codegen hook");
    let mut state = state::load(&ctx.deploy.compile.project_root);
    state
        .codegen_start
        .insert(ctx.deploy.contract_name, state::now());
    state::save(&ctx.deploy.compile.project_root, &state);
}

fn cmd_post_codegen() {
    let ctx = read_stdin::<CodegenContext>();
    let mut state = state::load(&ctx.deploy.compile.project_root);
    let mut reporter = Reporter::new(log_path(&ctx.deploy.compile.project_root).as_deref());
    reporter.log("📋 post codegen hook");

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
        "📋 Codegen {}:\n    duration = {}\n    package size = {}",
        ctx.deploy.contract_name, elapsed, ts_size_kb,
    ));

    state::save(&ctx.deploy.compile.project_root, &state);
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
    let ctx = read_stdin::<ProjectContext>();
    let mut state = state::load(&ctx.project_root);
    state.dev_start = Some(state::now());
    state::save(&ctx.project_root, &state);
}

fn cmd_post_dev() {
    let ctx = read_stdin::<ProjectContext>();
    let mut state = state::load(&ctx.project_root);
    let mut reporter = Reporter::new(log_path(&ctx.project_root).as_deref());

    if let Some(start) = state.dev_start.take() {
        let elapsed = state::elapsed_since(start);
        let contract_count = ctx.contracts.len();
        reporter.log(&format!(
            "📋 build cycle complete: {contract_count} contract(s) in {elapsed:.2}s"
        ));
    }

    state::save(&ctx.project_root, &state);
}
