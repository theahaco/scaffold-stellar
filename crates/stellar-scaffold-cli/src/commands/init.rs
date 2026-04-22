use clap::{Args, Parser};
use degit::degit;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use std::fs::{copy, metadata, read_dir, remove_dir_all, remove_file, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, io};

use super::{build, generate};
use crate::commands::{PackageManager, PackageManagerSpec};
use stellar_cli::{commands::global, print::Print};

pub const FRONTEND_TEMPLATE: &str = "theahaco/scaffold-stellar-frontend";
const TUTORIAL_BRANCH: &str = "tutorial";
const PNPM_WORKSPACE: &str = r#"packages:
  - "packages/*"
"#;

/// A command to initialize a new project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the project must be provided
    pub project_path: PathBuf,

    #[command(flatten)]
    vers: Vers,
}

#[derive(Args, Debug, Clone)]
#[group(multiple = false)]
struct Vers {
    /// Initialize the tutorial project instead of the default project
    #[arg(long, default_value_t = false)]
    pub tutorial: bool,

    /// Optional argument to specify a tagged version
    #[arg(long)]
    pub tag: Option<String>,
}

/// Errors that can occur during initialization
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to clone template: {0}")]
    DegitError(String),
    #[error("Project path contains invalid UTF-8 characters and cannot be converted to a string")]
    InvalidProjectPathEncoding,
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error(transparent)]
    GenerateError(#[from] generate::contract::Error),
}

impl Cmd {
    /// Run the initialization command
    ///
    /// # Example:
    ///
    /// ```
    /// /// From the command line
    /// stellar scaffold init /path/to/project
    /// ```
    #[allow(clippy::too_many_lines)]
    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer: Print = Print::new(global_args.quiet);

        // Convert to absolute path to avoid issues when changing directories
        let absolute_project_path = self.project_path.canonicalize().unwrap_or_else(|_| {
            // If canonicalize fails (path doesn't exist yet), manually create absolute path
            if self.project_path.is_absolute() {
                self.project_path.clone()
            } else {
                env::current_dir()
                    .unwrap_or_default()
                    .join(&self.project_path)
            }
        });

        printer.infoln(format!(
            "Creating new Stellar project in {}",
            absolute_project_path.display()
        ));

        let project_str = absolute_project_path
            .to_str()
            .ok_or(Error::InvalidProjectPathEncoding)?
            .to_owned();

        let mut repo = FRONTEND_TEMPLATE.to_string();
        if let Some(tag) = self.vers.tag.as_deref() {
            repo = format!("{repo}#{tag}");
        } else if self.vers.tutorial {
            repo = format!("{repo}#{TUTORIAL_BRANCH}");
        }
        tokio::task::spawn_blocking(move || {
            degit(repo.as_str(), &project_str);
        })
        .await
        .expect("Blocking task panicked");

        if metadata(&absolute_project_path).is_err()
            || read_dir(&absolute_project_path)?.next().is_none()
        {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {}: directory is empty or missing",
                absolute_project_path.display()
            )));
        }

        // Copy .env.example to .env
        let example_path = absolute_project_path.join(".env.example");
        let env_path = absolute_project_path.join(".env");
        copy(example_path, env_path)?;

        // Update the project's OpenZeppelin examples with the latest editions
        if !self.vers.tutorial {
            let example_contracts = ["oz/nft-enumerable", "oz/fungible-allowlist"];

            for contract in example_contracts {
                self.update_oz_example(&absolute_project_path, contract, global_args)
                    .await?;
            }
        }

        let Some(pkg_manager) = select_pkg_manager(&printer) else {
            printer.warnln("Package manager selection cancelled. Run the command again to retry.");
            return Ok(());
        };

        if pkg_manager.kind == PackageManager::Pnpm {
            if let Err(e) = write(
                absolute_project_path.join("pnpm-workspace.yaml"),
                PNPM_WORKSPACE,
            ) {
                printer.warnln(format!("Failed to create pnpm-workspace.yaml: {e}"));
            }
        } else if pkg_manager.kind != PackageManager::Npm
            && let Err(e) = remove_file(absolute_project_path.join("package-lock.json"))
        {
            printer.warnln(format!("Failed to remove package-lock.json: {e}"));
        }

        if pkg_manager
            .write_to_package_json(&absolute_project_path)
            .is_err()
        {
            printer.warnln("Failed to write the selected package manager to package.json");
        }

        // Install dependencies
        let pm_command = pkg_manager.kind.command();
        let install_succeeded = run_install(pm_command, &absolute_project_path, &printer);

        // Build contracts and create contract clients
        printer.infoln("Building contracts and generating client code...");
        // Use clap to parse build command with defaults, then configure programmatically
        let mut build_command = build::Command::parse_from(["build", "--build-clients"]);
        build_command.build.manifest_path = Some(absolute_project_path.join("Cargo.toml"));
        build_command.build_clients_args.env = Some(build::clients::ScaffoldEnv::Development);
        build_command.build_clients_args.workspace_root = Some(absolute_project_path.clone());
        let mut build_args = global_args.clone();
        if !(global_args.verbose && global_args.very_verbose) {
            build_args.quiet = true;
        }

        if let Err(e) = build_command.run(&build_args).await {
            printer.warnln(format!("Failed to build contract clients: {e}"));
        }

        // If git is installed, run init and make initial commit
        if git_exists() {
            git_init(&absolute_project_path);
            git_add(&absolute_project_path, &["-A"]);
            git_commit(&absolute_project_path, "initial commit");
        }

        printer.blankln("\n\n");
        printer.checkln(format!(
            "Project successfully created at {}!",
            absolute_project_path.display()
        ));
        printer.blankln(" You can now run the application with:\n");
        printer.blankln(format!("\tcd {}", self.project_path.display()));
        if !install_succeeded {
            printer.blankln(format!("\t{pm_command} install"));
        }
        printer.blankln(format!("\t{pm_command} start"));
        printer.blankln(" Happy hacking! 🚀");
        Ok(())
    }

    /// Updates the project with an Open Zeppelin example contract
    ///
    /// This method attempts to generate a contract from Open Zeppelin
    /// and prints a warning if it can't be found or generated.
    async fn update_oz_example(
        &self,
        absolute_project_path: &Path,
        example_name: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let mut example_path = example_name;
        if example_name.starts_with("oz/") {
            (_, example_path) = example_name.split_at(3);
        }

        let printer = Print::new(global_args.quiet);
        let original_dir = env::current_dir()?;
        env::set_current_dir(absolute_project_path)?;

        let all_contracts_path = absolute_project_path.join("contracts");
        let existing_contract_path = all_contracts_path.join(example_path);

        if existing_contract_path.exists() {
            remove_dir_all(&existing_contract_path)?;
        }

        let mut quiet_global_args = global_args.clone();
        quiet_global_args.quiet = false;

        let result = generate::contract::Cmd {
            from: Some(example_name.to_owned()),
            ls: false,
            from_wizard: false,
            output: Some(all_contracts_path.join(example_path)),
            force: false,
        }
        .run(&quiet_global_args)
        .await;

        // Restore directory before handling result
        let _ = env::set_current_dir(original_dir);

        match result {
            Ok(()) => {
                printer.infoln(format!(
                    "Successfully added OpenZeppelin example contract: {example_path}"
                ));
            }
            Err(generate::contract::Error::OzExampleNotFound(_)) => {
                printer.infoln(format!(
                    "Skipped missing OpenZeppelin example contract: {example_path}"
                ));
            }
            Err(e) => {
                printer.warnln(format!(
                    "Failed to generate example contract: {example_path}\n{e}"
                ));
            }
        }

        Ok(())
    }
}

/// Probe all known package managers and return specs for those that are installed.
fn detect_pkg_managers() -> Vec<PackageManagerSpec> {
    PackageManager::LIST
        .iter()
        .filter_map(|kind| {
            let version = pkg_manager_version(kind.command())?;
            Some(PackageManagerSpec {
                kind: kind.clone(),
                version: Some(version),
            })
        })
        .collect()
}

/// Interactively pick a package manager. Shows only installed managers with their
/// detected versions. Defaults to npm if available, otherwise the first detected.
/// Returns `None` if the user cancels (Ctrl+C) or no managers are found.
fn select_pkg_manager(printer: &Print) -> Option<PackageManagerSpec> {
    let detected = detect_pkg_managers();

    if detected.is_empty() {
        printer.warnln("No supported package manager detected (npm, pnpm, yarn, bun, deno).");
        printer.warnln("Defaulting to npm — install it from https://nodejs.org");
        return Some(PackageManagerSpec {
            kind: PackageManager::Npm,
            version: None,
        });
    }

    if detected.len() == 1 {
        let spec = detected.into_iter().next().unwrap();
        let label = format_pm_label(&spec);
        printer.infoln(format!("Using {label} (only package manager detected)"));
        return Some(spec);
    }

    // Default selection: prefer npm, otherwise use the first detected manager
    let default_index = detected
        .iter()
        .position(|s| s.kind == PackageManager::Npm)
        .unwrap_or(0);

    let labels: Vec<String> = detected.iter().map(format_pm_label).collect();

    let index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a package manager")
        .items(&labels)
        .default(default_index)
        .interact()
        .ok()?;

    detected.into_iter().nth(index)
}

fn format_pm_label(spec: &PackageManagerSpec) -> String {
    match &spec.version {
        Some(v) => format!("{} ({})", spec.kind.as_str(), v),
        None => spec.kind.as_str().to_string(),
    }
}

fn pkg_manager_version(command: &str) -> Option<String> {
    let output = Command::new(command).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_version(&stdout)
}

fn run_install(pm_command: &str, path: &Path, printer: &Print) -> bool {
    if pkg_manager_version(pm_command).is_none() {
        printer.warnln(format!(
            "Failed to install dependencies, {pm_command} is not installed"
        ));
        return false;
    }

    printer.infoln("Installing dependencies...");
    match Command::new(pm_command)
        .arg("install")
        .current_dir(path)
        .output()
    {
        Ok(output) if output.status.success() => true,
        Ok(output) => {
            printer.warnln(format!(
                "Failed to install dependencies: Please run '{pm_command} install' manually"
            ));
            if !output.stderr.is_empty()
                && let Ok(stderr) = String::from_utf8(output.stderr)
            {
                printer.warnln(format!("Error: {}", stderr.trim()));
            }
            false
        }
        Err(e) => {
            printer.warnln(format!("Failed to run {pm_command} install: {e}"));
            false
        }
    }
}

// Check if git is installed and exists in PATH
fn git_exists() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

fn git_init(path: &Path) {
    let _ = Command::new("git").arg("init").current_dir(path).output();
}

fn git_add(path: &Path, rest: &[&str]) {
    let mut args = vec!["add"];
    args.extend_from_slice(rest);
    let _ = Command::new("git").args(args).current_dir(path).output();
}

fn git_commit(path: &Path, message: &str) {
    let _ = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .output();
}

fn extract_version(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        if is_semver_like(token) {
            return Some(
                token
                    .trim_matches(|c: char| !c.is_ascii_digit() && c != '.')
                    .to_string(),
            );
        }
    }
    None
}

fn is_semver_like(s: &str) -> bool {
    let s = s.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
    let mut parts = s.split('.');

    let major = parts.next().and_then(|p| p.parse::<u64>().ok());
    let minor = parts.next().and_then(|p| p.parse::<u64>().ok());

    // patch is optional (yarn classic sometimes omits weirdly)
    let patch = parts.next().map_or(Some(0), |p| p.parse::<u64>().ok());

    major.is_some() && minor.is_some() && patch.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_from_npm_output() {
        assert_eq!(extract_version("10.2.3"), Some("10.2.3".to_string()));
    }

    #[test]
    fn extract_version_from_pnpm_output() {
        // pnpm --version outputs just the version number
        assert_eq!(extract_version("9.6.0"), Some("9.6.0".to_string()));
    }

    #[test]
    fn extract_version_from_yarn_output() {
        // yarn --version outputs just the version
        assert_eq!(extract_version("1.22.19"), Some("1.22.19".to_string()));
    }

    #[test]
    fn extract_version_from_prefixed_string() {
        // some tools prefix with 'v'
        assert_eq!(extract_version("v1.2.3"), Some("1.2.3".to_string()));
    }

    #[test]
    fn extract_version_ignores_non_version_tokens() {
        assert_eq!(extract_version("npm 10.2.3"), Some("10.2.3".to_string()));
    }

    #[test]
    fn extract_version_returns_none_for_garbage() {
        assert_eq!(extract_version("not-a-version"), None);
    }

    #[test]
    fn is_semver_like_two_part_accepted() {
        // yarn classic sometimes shows only major.minor
        assert!(is_semver_like("1.22"));
    }

    #[test]
    fn is_semver_like_three_part() {
        assert!(is_semver_like("10.2.3"));
    }

    #[test]
    fn is_semver_like_rejects_word() {
        assert!(!is_semver_like("npm"));
    }

    #[test]
    fn is_semver_like_rejects_single_number() {
        assert!(!is_semver_like("10"));
    }
}
