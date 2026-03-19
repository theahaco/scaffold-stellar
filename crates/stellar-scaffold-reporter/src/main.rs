use clap::{Parser, Subcommand};
use stellar_scaffold_ext_types::{CompileContext, ExtensionManifest, HookName};
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
    // PreDeploy,
    // PostDeploy,
    // PreCodegen,
    // PostCodegen,
    // PreDev,
    // PostDev,
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
        // Command::PreDeploy => cmd_pre_deploy(),
        // Command::PostDeploy => cmd_post_deploy(),
        // Command::PreCodegen => cmd_pre_codegen(),
        // Command::PostCodegen => cmd_post_codegen(),
        // Command::PreDev => cmd_pre_dev(),
        // Command::PostDev => cmd_post_dev(),
    }
}

fn cmd_manifest() {
    let manifest = ExtensionManifest {
        name: String::from("reporter"),
        version: String::from(env!("CARGO_PKG_VERSION")),
        hooks: [
            HookName::PreCompile,
            HookName::PostCompile,
            // HookName::PreDeploy,
            // HookName::PostDeploy,
            // HookName::PreCodegen,
            // HookName::PostCodegen,
            // HookName::PreDev,
            // HookName::PostDev,
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
    let ctx: CompileContext = read_stdin();
    let mut state = state::load(&ctx.project_root);
    state.compile_start = Some(state::now());
    state::save(&ctx.project_root, &state);
}

fn cmd_post_compile() {
    let ctx: CompileContext = read_stdin();
    let mut state = state::load(&ctx.project_root);
    let mut reporter = report::Reporter::new(log_path(&ctx.project_root).as_deref());

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
