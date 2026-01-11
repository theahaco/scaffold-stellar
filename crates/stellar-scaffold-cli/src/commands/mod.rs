use std::{
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
    str::FromStr,
};

use clap::{CommandFactory, FromArgMatches, Parser, command};
use serde_json::Value;
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
    #[serde(rename = "packageManager")]
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

    pub fn write_to_package_json(&self, workspace_root: &Path) -> io::Result<()> {
        let pkg_path = workspace_root.join("package.json");
        let contents =
            read_to_string(&pkg_path).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut value: Value = serde_json::from_str(&contents)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let pacman_value = match &self.version {
            Some(version) => format!("{}@{}", self.kind.as_str(), version),
            None => self.kind.as_str().to_string(),
        };

        value["packageManager"] = Value::String(pacman_value);

        let updated = serde_json::to_string_pretty(&value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        std::fs::write(&pkg_path, updated)?;
        Ok(())
    }

    pub fn from_package_json(workspace_root: &Path) -> Option<Self> {
        let pkg_path = workspace_root.join("package.json");
        let contents = read_to_string(pkg_path).ok()?;

        let pkg: PackageJson = serde_json::from_str(&contents).ok()?;
        let raw = pkg.package_manager?;

        Some(PackageManagerSpec::parse_package_manager_field(&raw))
    }

    // "pnpm@9.6.0" → ("pnpm", "9.6.0")
    fn parse_package_manager_field(value: &str) -> Self {
        let mut parts = value.split('@');
        let name = parts.next().unwrap_or(value);
        let version = parts.next().map(std::string::ToString::to_string);

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
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Deno,
}

impl PackageManager {
    pub const LIST: &'static [Self] = &[Self::Npm, Self::Pnpm, Self::Yarn, Self::Bun, Self::Deno];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }

    pub fn command(&self) -> &'static str {
        match self {
            Self::Npm => Self::os_specific_command("npm"),
            Self::Pnpm => Self::os_specific_command("pnpm"),
            Self::Yarn => Self::os_specific_command("yarn"),
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }

    fn os_specific_command(base: &'static str) -> &'static str {
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

    fn install_silent(&self, dir: &PathBuf) -> io::Result<Output> {
        let mut cmd = Command::new(self.command());
        cmd.current_dir(dir);

        match self {
            Self::Npm => cmd.args(["install", "--loglevel=error"]),
            Self::Pnpm | Self::Yarn => cmd.args(["install", "--silent"]),
            _ => cmd.args(["install"]),
        };

        cmd.output()
    }

    fn install_no_workspace(&self, dir: &PathBuf) -> io::Result<Output> {
        let mut cmd = Command::new(self.command());
        cmd.current_dir(dir);

        match self {
            Self::Npm => cmd.args(["install", "--no-workspaces", "--loglevel=error"]),
            Self::Pnpm | Self::Yarn => cmd.args(["install", "--silent"]),
            _ => cmd.args(["install"]),
        };

        cmd.output()
    }

    fn build(&self, dir: &PathBuf) -> io::Result<Output> {
        Command::new(self.command())
            .current_dir(dir)
            .arg("run")
            .arg("build")
            .output()
    }
}
