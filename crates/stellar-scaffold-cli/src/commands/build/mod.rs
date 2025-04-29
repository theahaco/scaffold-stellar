#![allow(clippy::struct_excessive_bools)]
use cargo_metadata::{Metadata, MetadataCommand, Package};
use clap::Parser;
use itertools::Itertools;
use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    fmt::Debug,
    fs, io,
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

pub mod clients;
pub mod env_toml;

/// Build a contract from source
///
/// Builds all crates that are referenced by the cargo manifest (Cargo.toml)
/// that have cdylib as their crate-type. Crates are built for the wasm32
/// target. Unless configured otherwise, crates are built with their default
/// features and with their release profile.
///
/// To view the commands that will be executed, without executing them, use the
/// --print-commands-only option.
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// List package names
    #[arg(long, visible_alias = "ls")]
    pub list: bool,
    /// Path to Cargo.toml
    #[arg(long, default_value = "Cargo.toml")]
    pub manifest_path: std::path::PathBuf,
    /// Package to build
    ///
    /// If omitted, all packages that build for crate-type cdylib are built.
    #[arg(long)]
    pub package: Option<String>,
    /// Build with the specified profile
    #[arg(long)]
    pub profile: Option<String>,
    /// Build with the list of features activated, space or comma separated
    #[arg(long, help_heading = "Features")]
    pub features: Option<String>,
    /// Build with the all features activated
    #[arg(
        long,
        conflicts_with = "features",
        conflicts_with = "no_default_features",
        help_heading = "Features"
    )]
    pub all_features: bool,
    /// Build with the default feature not activated
    #[arg(long, help_heading = "Features")]
    pub no_default_features: bool,
    /// Directory to copy wasm files to
    ///
    /// If provided, wasm files can be found in the cargo target directory, and
    /// the specified directory.
    ///
    /// If ommitted, wasm files are written only to `target/stellar`.
    #[arg(long)]
    pub out_dir: Option<std::path::PathBuf>,
    /// Print commands to build without executing them
    #[arg(long, conflicts_with = "out_dir", help_heading = "Other")]
    pub print_commands_only: bool,
    /// Build client code in addition to building the contract
    #[arg(long)]
    pub build_clients: bool,
    #[command(flatten)]
    pub build_clients_args: clients::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Metadata(#[from] cargo_metadata::Error),
    #[error(transparent)]
    CargoCmd(io::Error),
    #[error("exit status {0}")]
    Exit(ExitStatus),
    #[error("package {package} not found")]
    PackageNotFound { package: String },
    #[error("creating out directory: {0}")]
    CreatingOutDir(io::Error),
    #[error("copying wasm file: {0}")]
    CopyingWasmFile(io::Error),
    #[error("getting the current directory: {0}")]
    GettingCurrentDir(io::Error),
    #[error(transparent)]
    StellarBuild(#[from] stellar_build::deps::Error),
    #[error(transparent)]
    BuildClients(#[from] clients::Error),
}

impl Cmd {
    pub fn list_packages(&self) -> Result<Vec<Package>, Error> {
        let metadata = self.metadata()?;
        let packages = self.packages(&metadata)?;
        Ok(stellar_build::deps::get_workspace(&packages)?)
    }

    pub async fn run(&self) -> Result<(), Error> {
        let working_dir = env::current_dir().map_err(Error::GettingCurrentDir)?;
        let metadata = self.metadata()?;
        let packages = self.list_packages()?;
        if self.list {
            for p in packages {
                println!("{}", p.name);
            }
            return Ok(());
        }
        let target_dir = &metadata.target_directory;

        if let Some(package) = &self.package {
            if packages.is_empty() {
                return Err(Error::PackageNotFound {
                    package: package.clone(),
                });
            }
        }

        let mut package_names: Vec<String> = Vec::new();
        for p in packages {
            package_names.push(p.name.clone().replace('-', "_"));
            let mut cmd = Command::new("cargo");
            cmd.stdout(Stdio::piped());
            cmd.arg("rustc");
            let manifest_path = pathdiff::diff_paths(&p.manifest_path, &working_dir)
                .unwrap_or(p.manifest_path.clone().into());
            cmd.arg(format!(
                "--manifest-path={}",
                manifest_path.to_string_lossy()
            ));
            cmd.arg("--crate-type=cdylib");
            cmd.arg("--target=wasm32-unknown-unknown");
            let profile = self.profile.as_deref().unwrap_or("release");
            if profile == "release" {
                cmd.arg("--release");
            } else if profile != "debug" {
                cmd.arg(format!("--profile={profile}"));
            }
            if self.all_features {
                cmd.arg("--all-features");
            }
            if self.no_default_features {
                cmd.arg("--no-default-features");
            }
            if let Some(features) = self.features() {
                let requested: HashSet<String> = features.iter().cloned().collect();
                let available = p.features.iter().map(|f| f.0).cloned().collect();
                let activate = requested.intersection(&available).join(",");
                if !activate.is_empty() {
                    cmd.arg(format!("--features={activate}"));
                }
            }
            if self.profile.is_none() {
                set_default_profile_flags(&mut cmd);
            }
            let cmd_str = format!(
                "cargo {}",
                cmd.get_args().map(OsStr::to_string_lossy).join(" ")
            );

            if self.print_commands_only {
                println!("{cmd_str}");
            } else {
                eprintln!("{cmd_str}");
                let status = cmd.status().map_err(Error::CargoCmd)?;
                if !status.success() {
                    return Err(Error::Exit(status));
                }

                let out_dir = self
                    .out_dir
                    .clone()
                    .map_or_else(|| stellar_build::deps::get_target_dir(&manifest_path), Ok)?;

                fs::create_dir_all(&out_dir).map_err(Error::CreatingOutDir)?;
                let file = format!("{}.wasm", p.name.replace('-', "_"));
                let target_file_path = Path::new(target_dir)
                    .join("wasm32-unknown-unknown")
                    .join(profile)
                    .join(&file);
                let out_file_path = out_dir.join(&file);
                if !out_file_path.exists() {
                    symlink::symlink_file(target_file_path, out_file_path)
                        .map_err(Error::CopyingWasmFile)?;
                }
            }
        }

        if self.build_clients {
            self.build_clients_args
                .run(&metadata.workspace_root.into_std_path_buf(), package_names)
                .await?;
        }

        Ok(())
    }

    fn features(&self) -> Option<Vec<String>> {
        self.features
            .as_ref()
            .map(|f| f.split(&[',', ' ']).map(String::from).collect())
    }

    fn packages(&self, metadata: &Metadata) -> Result<Vec<Package>, Error> {
        if let Some(package) = &self.package {
            let package = metadata
                .packages
                .iter()
                .find(|p| p.name == *package)
                .ok_or_else(|| Error::PackageNotFound {
                    package: package.clone(),
                })?
                .clone();
            let manifest_path = package.manifest_path.clone().into_std_path_buf();
            let mut contracts = stellar_build::deps::contract(&manifest_path)?;
            contracts.push(package);
            return Ok(contracts);
        }
        Ok(metadata
            .packages
            .iter()
            .filter(|p| {
                // Filter crates by those that build to cdylib (wasm)
                p.targets
                    .iter()
                    .any(|t| t.crate_types.iter().any(|c| c == "cdylib"))
            })
            .cloned()
            .collect())
    }

    fn metadata(&self) -> Result<Metadata, cargo_metadata::Error> {
        let mut cmd = MetadataCommand::new();
        cmd.no_deps();
        cmd.manifest_path(&self.manifest_path);
        // Do not configure features on the metadata command, because we are
        // only collecting non-dependency metadata, features have no impact on
        // the output.
        cmd.exec()
    }
}

fn set_default_profile_flags(cmd: &mut Command) {
    cmd.args([
        "--",
        "-C",
        "opt-level=z", // Sets the optimization level to "z", which is equivalent to the opt-level = "z" in the Cargo profile.
        "-C",
        "overflow-checks=yes", // Enables overflow checks, equivalent to overflow-checks = true.
        "-C",
        "debuginfo=0", // Disables debug information, equivalent to debug = 0.
        "-C",
        "strip=symbols", // Strips symbols from the binary, equivalent to strip = "symbols".
        "-C",
        "debug-assertions=yes", // Enables debug assertions, equivalent to debug-assertions = true.
        "-C",
        "panic=abort", // Sets the panic strategy to "abort", equivalent to panic = "abort".
        "-C",
        "codegen-units=1", // Sets the number of codegen units to 1, equivalent to codegen-units = 1.
        "-C",
        "lto=yes", // Enables link-time optimization, equivalent to lto = true.
    ]);
    cmd.env("RUSTFLAGS", "-C embed-bitcode=yes");
}
