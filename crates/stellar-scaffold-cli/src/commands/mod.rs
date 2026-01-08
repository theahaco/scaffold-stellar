use std::{path::PathBuf, str::FromStr};

use clap::{CommandFactory, FromArgMatches, Parser, command};
use stellar_cli;

pub mod build;
pub mod generate;
pub mod init;
pub mod update_env;
pub mod upgrade;
pub mod version;
pub mod watch;

const ABOUT: &str = "Build smart contracts with frontend support";

#[derive(Parser, Debug)]
#[command(
    name = "stellar-scaffold",
    about = ABOUT,
    disable_help_subcommand = true,
)]
pub struct Root {
    #[clap(flatten)]
    pub global_args: stellar_cli::commands::global::Args,

    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Root {
    pub fn new() -> Result<Self, clap::Error> {
        let mut matches = Self::command().get_matches();
        Self::from_arg_matches_mut(&mut matches)
    }

    pub fn from_arg_matches<I, T>(itr: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self::from_arg_matches_mut(&mut Self::command().get_matches_from(itr))
    }
    pub async fn run(&mut self) -> Result<(), Error> {
        match &mut self.cmd {
            Cmd::Init(init_info) => init_info.run(&self.global_args).await?,
            Cmd::Version(version_info) => version_info.run(),
            Cmd::Build(build_info) => build_info.run(&self.global_args).await?,
            Cmd::Generate(generate) => match &mut generate.cmd {
                generate::Command::Contract(contract) => contract.run(&self.global_args).await?,
            },
            Cmd::Upgrade(upgrade_info) => upgrade_info.run(&self.global_args).await?,
            Cmd::UpdateEnv(e) => e.run()?,
            Cmd::Watch(watch_info) => watch_info.run(&self.global_args).await?,
        }
        Ok(())
    }
}

impl FromStr for Root {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_arg_matches(s.split_whitespace())
    }
}

#[derive(Parser, Debug)]
pub enum Cmd {
    /// Initialize the project
    Init(init::Cmd),
    /// Version of the scaffold-stellar-cli
    Version(version::Cmd),

    /// Build contracts, resolving dependencies in the correct order. If you have an `environments.toml` file, it will also follow its instructions to configure the environment set by the `STELLAR_SCAFFOLD_ENV` environment variable, turning your contracts into frontend packages (JS dependencies).
    Build(build::Command),

    /// generate contracts
    Generate(generate::Cmd),

    /// Upgrade an existing Soroban workspace to a scaffold project
    Upgrade(upgrade::Cmd),

    /// Update an environment variable in a .env file
    UpdateEnv(update_env::Cmd),

    /// Monitor contracts and environments.toml for changes and rebuild as needed
    Watch(watch::Cmd),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // TODO: stop using Debug for displaying errors
    #[error(transparent)]
    Init(#[from] init::Error),
    #[error(transparent)]
    BuildContracts(#[from] build::Error),
    #[error(transparent)]
    Contract(#[from] generate::contract::Error),
    #[error(transparent)]
    Upgrade(#[from] upgrade::Error),
    #[error(transparent)]
    UpdateEnv(#[from] update_env::Error),
    #[error(transparent)]
    Watch(#[from] watch::Error),
}

#[derive(serde::Deserialize)]
struct PackageJson {
    #[serde(rename = "packageManger")]
    package_manager: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackageManagerSpec {
    pub kind: PackageManager,
    pub version: Option<String>,
}

impl PackageManagerSpec {
    pub fn command(&self) -> &'static str {
        self.kind.command()
    }

    pub fn from_package_json(workspace_root: &PathBuf) -> Result<Self, ()> {
        let pkg_path = workspace_root.join("package.json");
        let contents = read_to_string(pkg_path)?;
        let pkg: PackageJson = serde_json::from_str(&contents)?;
        let raw = pkg.package_manager.ok_or(())?;
        
        Ok(PackageManagerSpec::parse_package_manager_field(&raw))
    }

    // "pnpm@9.6.0" → ("pnpm", "9.6.0")
    fn parse_package_manager_field(value: &str) -> Self {
        let mut parts = value.split('@');
        let name = parts.next().unwrap_or(value);
        let version = parts.next().map(|v| v.to_string());

        let kind = match name {
            "pnpm" => PackageManager::Pnpm,
            "yarn" => PackageManager::Yarn,
            "bun" => PackageManager::Bun,
            "deno" => PackageManager::Deno,
            _ => PackageManager::Npm,
        };

        Self { kind, version }
    }
}

#[derive(Debug, Clone)]
enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Deno,
}

impl PackageManager {
    pub const LIST: &'static [Self] = &[Self::Npm, Self::Pnpm, Self::Yarn, Self::Bun, Self::Deno];

    pub fn command(&self) -> &'static str {
        match self {
            Self::Npm => self.os_specific_command("npm"),
            Self::Pnpm => self.os_specific_command("pnpm"),
            Self::Yarn => self.os_specific_command("yarn"),
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }

    fn os_specfic_command(base: &'static str) -> &'static str {
        if cfg!(target_os = "windows") {
            match base {
                "npm" => "npm.cmd",
                "pnpm" => "pnpm.cmd",
                "yarn" => "yarn.cmd",
                _ => base,
            }
        } else {
            base
        }
    }
}
