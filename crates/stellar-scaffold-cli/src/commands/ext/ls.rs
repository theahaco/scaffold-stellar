use cargo_metadata::MetadataCommand;
use clap::Parser;
use stellar_cli::print::Print;

use crate::commands::build::clients::ScaffoldEnv;
use crate::commands::build::env_toml;
use crate::extension::{ExtensionListStatus, list};

const H_NAME: &str = "NAME";
const H_VERSION: &str = "VERSION";
const H_STATUS: &str = "STATUS";
const H_HOOKS: &str = "HOOKS";

#[derive(Parser, Debug)]
pub struct Cmd {
    /// Scaffold environment whose extension list to inspect.
    #[arg(
        env = "STELLAR_SCAFFOLD_ENV",
        value_enum,
        default_value = "development"
    )]
    pub env: ScaffoldEnv,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Metadata(#[from] cargo_metadata::Error),
    #[error(transparent)]
    Env(#[from] env_toml::Error),
}

impl Cmd {
    pub fn run(&self, global_args: &stellar_cli::commands::global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        let metadata = MetadataCommand::new().no_deps().exec()?;
        let workspace_root = metadata.workspace_root.as_std_path();

        let Some(current_env) = env_toml::Environment::get(workspace_root, &self.env)? else {
            printer.warnln(format!(
                "No environments.toml found or no {:?} environment configured.",
                self.env
            ));
            return Ok(());
        };

        if current_env.extensions.is_empty() {
            printer.infoln(format!(
                "No extensions configured for the {:?} environment.",
                self.env
            ));
            return Ok(());
        }

        let entries = list(&current_env.extensions);

        // Compute column widths (at least as wide as the header label).
        let name_w = entries
            .iter()
            .map(|e| e.name.len())
            .max()
            .unwrap_or(0)
            .max(H_NAME.len());
        let version_w = entries
            .iter()
            .map(|e| match &e.status {
                ExtensionListStatus::Found { version, .. } => version.len(),
                _ => 1, // "-"
            })
            .max()
            .unwrap_or(0)
            .max(H_VERSION.len());
        let status_w = entries
            .iter()
            .map(|e| status_str(&e.status).len())
            .max()
            .unwrap_or(0)
            .max(H_STATUS.len());

        // Header + separator.
        println!("{H_NAME:<name_w$}  {H_VERSION:<version_w$}  {H_STATUS:<status_w$}  {H_HOOKS}",);
        println!(
            "{:-<name_w$}  {:-<version_w$}  {:-<status_w$}  {:-<hooks_w$}",
            "",
            "",
            "",
            "",
            hooks_w = H_HOOKS.len(),
        );

        // Rows.
        for entry in &entries {
            let version = match &entry.status {
                ExtensionListStatus::Found { version, .. } => version.as_str(),
                _ => "-",
            };
            let hooks = match &entry.status {
                ExtensionListStatus::Found { hooks, .. } if !hooks.is_empty() => hooks.join(", "),
                _ => "-".to_string(),
            };
            let name = &entry.name;
            let status = status_str(&entry.status);
            println!("{name:<name_w$}  {version:<version_w$}  {status:<status_w$}  {hooks}");
        }

        Ok(())
    }
}

fn status_str(status: &ExtensionListStatus) -> &'static str {
    match status {
        ExtensionListStatus::Found { .. } => "found",
        ExtensionListStatus::MissingBinary => "missing",
        ExtensionListStatus::ManifestError(_) => "error",
    }
}
