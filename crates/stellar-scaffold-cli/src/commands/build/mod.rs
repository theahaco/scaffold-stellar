#![allow(clippy::struct_excessive_bools)]
use std::{fmt::Debug, io, path::Path, process::ExitStatus};

use cargo_metadata::{Metadata, MetadataCommand, Package};
use clap::Parser;
use stellar_cli::commands::{contract::build, global};

use clients::ScaffoldEnv;

pub mod clients;
pub mod docker;
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
pub struct Command {
    /// List package names in order of build
    #[arg(long, visible_alias = "ls")]
    pub list: bool,
    #[command(flatten)]
    pub build: build::Cmd,
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
    EnvironmentsToml(#[from] env_toml::Error),
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
    #[error(transparent)]
    Build(#[from] build::Error),
    #[error("Failed to start docker container")]
    DockerStart,
}

impl Command {
    pub fn list_packages(&self, metadata: &Metadata) -> Result<Vec<Package>, Error> {
        let packages = self.packages(metadata)?;
        Ok(stellar_build::deps::get_workspace(&packages)?)
    }

    async fn start_local_docker_if_needed(
        &self,
        workspace_root: &Path,
        env: &ScaffoldEnv,
    ) -> Result<(), Error> {
        if let Some(current_env) = env_toml::Environment::get(workspace_root, &env.to_string())? {
            if current_env.network.run_locally {
                eprintln!("Starting local Stellar Docker container...");
                docker::start_local_stellar().await.map_err(|e| {
                    eprintln!("Failed to start Stellar Docker container: {e:?}");
                    Error::DockerStart
                })?;
                eprintln!("Local Stellar network is healthy and running.");
            }
        }
        Ok(())
    }

    pub async fn run(&self) -> Result<(), Error> {
        let metadata = self.metadata()?;
        let packages = self.list_packages(&metadata)?;
        let workspace_root = metadata.workspace_root.as_std_path();

        if let Some(env) = &self.build_clients_args.env {
            if env == &ScaffoldEnv::Development {
                self.start_local_docker_if_needed(workspace_root, env)
                    .await?;
            }
        }

        if self.list {
            for p in packages {
                println!("{}", p.name);
            }
            return Ok(());
        }

        let target_dir = &metadata.target_directory;

        let global_args = global::Args::default();

        for p in &packages {
            let mut cmd = self.build.clone();
            cmd.out_dir = cmd
                .out_dir
                .or_else(|| Some(stellar_build::deps::stellar_wasm_out_dir(target_dir.as_std_path())));
            cmd.package = Some(p.name.clone());
            cmd.run(&global_args)?;
        }

        if self.build_clients {
            self.build_clients_args
                .run(
                    &metadata.workspace_root.into_std_path_buf(),
                    packages.iter().map(|p| p.name.replace('-', "_")).collect(),
                )
                .await?;
        }

        Ok(())
    }

    fn packages(&self, metadata: &Metadata) -> Result<Vec<Package>, Error> {
        if let Some(package) = &self.build.package {
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

    pub(crate) fn metadata(&self) -> Result<Metadata, cargo_metadata::Error> {
        let mut cmd = MetadataCommand::new();
        cmd.no_deps();
        // Set the manifest path if one is provided, otherwise rely on the cargo
        // commands default behavior of finding the nearest Cargo.toml in the
        // current directory, or the parent directories above it.
        if let Some(manifest_path) = &self.build.manifest_path {
            cmd.manifest_path(manifest_path);
        }
        // Do not configure features on the metadata command, because we are
        // only collecting non-dependency metadata, features have no impact on
        // the output.
        cmd.exec()
    }
}
