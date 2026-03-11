//! Shared types for stellar-scaffold extension hooks.
//!
//! Extensions are invoked as subprocesses. The scaffold tool serializes one of
//! these context types to JSON and writes it to the extension's stdin. The hook
//! name is communicated via the `STELLAR_SCAFFOLD_HOOK` environment variable.
//!
//! # Hook lifecycle
//!
//! ```text
//! pre-dev
//!   └─ pre-compile
//!        └─ [cargo build per contract]
//!   └─ post-compile
//!   └─ pre-deploy   (per contract)
//!        └─ [upload wasm, deploy/upgrade contract]
//!   └─ post-deploy  (per contract)
//!   └─ pre-codegen  (per contract)
//!        └─ [stellar contract bindings typescript + npm build]
//!   └─ post-codegen (per contract)
//! post-dev
//! ```
//!
//! Context types form a strict information superset going down the chain:
//! `CompileContext` ⊂ `DeployContext` ⊂ `CodegenContext`.
//! `ProjectContext` (used by `pre-dev`/`post-dev`) aggregates all per-contract
//! data into a single flat list.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ExtensionManifest
// ---------------------------------------------------------------------------

/// Declares what an extension is and which hooks it wants to receive.
///
/// Extensions must write this as JSON to stdout when invoked with
/// `STELLAR_SCAFFOLD_HOOK=manifest`, so the scaffold tool can discover
/// capabilities without running the full hook.
///
/// # Hook names
///
/// Valid values for `hooks` entries:
/// - `"pre-compile"` / `"post-compile"` — fired once per build, receives [`CompileContext`]
/// - `"pre-deploy"` / `"post-deploy"` — fired per contract, receives [`DeployContext`]
/// - `"pre-codegen"` / `"post-codegen"` — fired per contract, receives [`CodegenContext`]
/// - `"pre-dev"` / `"post-dev"` — fired per watch cycle, receives [`ProjectContext`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Extension name, e.g. `"my-audit-tool"`.
    pub name: String,
    /// `SemVer` version string, e.g. `"1.0.0"`.
    pub version: String,
    /// Hook names this extension wants to be called for.
    pub hooks: Vec<String>,
}

// ---------------------------------------------------------------------------
// ExtensionConfig
// ---------------------------------------------------------------------------

/// Arbitrary per-extension configuration, sourced from `environments.toml`.
///
/// The scaffold tool passes this opaque value to extensions alongside the hook
/// context. Extensions are responsible for deserializing it into their own
/// config struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig(pub serde_json::Value);

// ---------------------------------------------------------------------------
// NetworkConfig
// ---------------------------------------------------------------------------

/// Resolved network connection details.
///
/// Derived from `environments.toml`'s `[<env>.network]` section after
/// resolving any named network (e.g. `"testnet"`) to concrete URLs via
/// the stellar-cli network registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Fully resolved RPC endpoint URL.
    pub rpc_url: String,
    /// Network passphrase used for transaction signing and contract ID
    /// derivation.
    pub network_passphrase: String,
    /// Optional logical name (`"testnet"`, `"mainnet"`, etc.) as declared in
    /// `environments.toml`. `None` when the network was configured with an
    /// explicit `rpc_url`/`network_passphrase` pair rather than a name.
    pub network_name: Option<String>,
}

// ---------------------------------------------------------------------------
// CompileContext  (pre-compile / post-compile)
// ---------------------------------------------------------------------------

/// Context passed to `pre-compile` and `post-compile` hooks.
///
/// Fired once per build, covering all contracts in the workspace.
///
/// ## Field availability by hook
///
/// | Field | `pre-compile` | `post-compile` |
/// |---|---|---|
/// | `project_root` | ✓ | ✓ |
/// | `env` | ✓ | ✓ |
/// | `wasm_out_dir` | ✓ | ✓ |
/// | `source_dirs` | ✓ | ✓ |
/// | `wasm_paths` | empty | populated |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileContext {
    /// Absolute path to the Cargo workspace root (where `Cargo.toml` and
    /// `environments.toml` live).
    pub project_root: PathBuf,

    /// Active scaffold environment: `"development"`, `"testing"`,
    /// `"staging"`, or `"production"`.
    pub env: String,

    /// Directory where compiled WASM files are written.
    ///
    /// Typically `<project_root>/target/stellar/<STELLAR_NETWORK>/`.
    /// The `STELLAR_NETWORK` value defaults to `"local"` when the env var is
    /// not set.
    pub wasm_out_dir: PathBuf,

    /// Parent directories of each contract's `Cargo.toml`, in topological
    /// build order (dependencies before dependents).
    ///
    /// These are the directories passed to `cargo build` for each cdylib
    /// crate in the workspace.
    pub source_dirs: Vec<PathBuf>,

    /// Map from contract name (`snake_case`, matching the WASM filename stem)
    /// to its compiled WASM path.
    ///
    /// **Empty at `pre-compile`**; populated at `post-compile` once all
    /// `cargo build` invocations have succeeded.
    pub wasm_paths: BTreeMap<String, PathBuf>,
}

// ---------------------------------------------------------------------------
// DeployContext  (pre-deploy / post-deploy)
// ---------------------------------------------------------------------------

/// Context passed to `pre-deploy` and `post-deploy` hooks.
///
/// Fired once per contract. Includes all fields from [`CompileContext`] (via
/// `#[serde(flatten)]`) plus network and per-contract deployment details.
///
/// ## Field availability by hook
///
/// | Field | `pre-deploy` | `post-deploy` |
/// |---|---|---|
/// | All `CompileContext` fields | ✓ | ✓ |
/// | `network` | ✓ | ✓ |
/// | `contract_name` | ✓ | ✓ |
/// | `wasm_path` | ✓ | ✓ |
/// | `wasm_hash` | ✓ | ✓ |
/// | `contract_id` | `None` | `Some(…)` |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployContext {
    /// All compile-stage fields (project root, env, wasm paths, etc.).
    #[serde(flatten)]
    pub compile: CompileContext,

    /// Resolved network configuration for this environment.
    pub network: NetworkConfig,

    /// Name of the contract currently being processed (`snake_case`, matching
    /// the WASM filename stem and the key in `environments.toml`).
    pub contract_name: String,

    /// Absolute path to the compiled WASM file for this contract.
    pub wasm_path: PathBuf,

    /// Hex-encoded SHA-256 hash of the uploaded WASM bytecode.
    ///
    /// The WASM is uploaded to the network before the deploy step, so this
    /// hash is available at both `pre-deploy` and `post-deploy`.
    pub wasm_hash: String,

    /// Stellar contract address in strkey format (`C…`).
    ///
    /// `None` at `pre-deploy` (the contract has not yet been instantiated or
    /// confirmed to exist at this hash). `Some` at `post-deploy`, regardless
    /// of whether the contract was freshly deployed or upgraded in-place.
    pub contract_id: Option<String>,
}

// ---------------------------------------------------------------------------
// CodegenContext  (pre-codegen / post-codegen)
// ---------------------------------------------------------------------------

/// Context passed to `pre-codegen` and `post-codegen` hooks.
///
/// Fired once per contract, after the deploy step. Includes all fields from
/// [`DeployContext`] (via `#[serde(flatten)]`) plus TypeScript package paths.
///
/// The paths `ts_package_dir` and `src_template_path` are deterministic and
/// present at both hooks; the files they point to may not exist yet at
/// `pre-codegen`.
///
/// ## What codegen produces
///
/// 1. `stellar contract bindings typescript` generates a TS package into a
///    temp dir (`<project_root>/target/packages/<name>/`).
/// 2. After `npm install` + `npm run build`, the result is moved/merged into
///    `ts_package_dir` (`<project_root>/packages/<name>/`).
/// 3. A thin client wrapper is written to `src_template_path`
///    (`<project_root>/src/contracts/<name>.ts`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodegenContext {
    /// All deploy-stage fields (compile context, network, contract deploy
    /// info, etc.).
    #[serde(flatten)]
    pub deploy: DeployContext,

    /// Final TypeScript package directory: `<project_root>/packages/<name>/`.
    ///
    /// Contains `src/index.ts`, `dist/index.js`, `dist/index.d.ts`, and
    /// `package.json` after `post-codegen`.
    pub ts_package_dir: PathBuf,

    /// Path to the generated client wrapper:
    /// `<project_root>/src/contracts/<name>.ts`.
    ///
    /// This file is written at the end of the codegen step; it may not exist
    /// yet at `pre-codegen`.
    pub src_template_path: PathBuf,
}

// ---------------------------------------------------------------------------
// ProjectContext  (pre-dev / post-dev)
// ---------------------------------------------------------------------------

/// Per-contract summary used inside [`ProjectContext`].
///
/// All `Option` fields are `None` at `pre-dev` (before any build has run)
/// and populated at `post-dev` for contracts that were successfully processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContractInfo {
    /// Contract name (`snake_case`).
    pub name: String,

    /// Parent directory of the contract's `Cargo.toml`.
    pub source_dir: PathBuf,

    /// Compiled WASM path. `None` if the contract failed to compile.
    pub wasm_path: Option<PathBuf>,

    /// Hex WASM hash. `None` if the contract was not uploaded this cycle.
    pub wasm_hash: Option<String>,

    /// Stellar contract address. `None` if the contract was not deployed or
    /// the environment does not deploy contracts (staging/production with
    /// pinned IDs are still `Some`).
    pub contract_id: Option<String>,

    /// Final TypeScript package directory. `None` if `client = false` or
    /// codegen was not run.
    pub ts_package_dir: Option<PathBuf>,

    /// Thin client wrapper path. `None` if codegen was not run.
    pub src_template_path: Option<PathBuf>,
}

/// Context passed to `pre-dev` and `post-dev` hooks.
///
/// Fired once per watch cycle (or once for a non-watch build with
/// `--build-clients`). Aggregates all per-contract information into a single
/// flat list rather than nesting `CodegenContext` directly, since the
/// per-contract hooks fire sequentially inside a single build cycle.
///
/// ## Field availability by hook
///
/// | Field | `pre-dev` | `post-dev` |
/// |---|---|---|
/// | `project_root`, `env`, `wasm_out_dir` | ✓ | ✓ |
/// | `source_dirs`, `watch_paths` | ✓ | ✓ |
/// | `network` | ✓ if `--build-clients` | ✓ if `--build-clients` |
/// | `contracts[*].wasm_path` etc. | `None` | populated |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Absolute path to the Cargo workspace root.
    pub project_root: PathBuf,

    /// Active scaffold environment.
    pub env: String,

    /// Directory where compiled WASM files are written.
    pub wasm_out_dir: PathBuf,

    /// Parent directories of each contract's `Cargo.toml`, in topological
    /// build order.
    pub source_dirs: Vec<PathBuf>,

    /// Resolved network configuration.
    ///
    /// `None` when the build was invoked without `--build-clients` (i.e. no
    /// network interaction occurs).
    pub network: Option<NetworkConfig>,

    /// Per-contract summary for every cdylib package in the workspace.
    ///
    /// At `pre-dev` all `Option` fields inside each entry are `None`. At
    /// `post-dev` they are populated for contracts that were successfully
    /// compiled, deployed, and had clients generated.
    pub contracts: Vec<ProjectContractInfo>,

    /// Absolute paths being watched for changes (contract source directories
    /// and the workspace root for `environments.toml`).
    ///
    /// Empty in a one-shot build (non-watch mode); populated in `stellar
    /// scaffold watch`.
    pub watch_paths: Vec<PathBuf>,
}
