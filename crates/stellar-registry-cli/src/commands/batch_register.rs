use std::fmt::Write as _;

use clap::Parser;
use stellar_cli::{commands::contract::invoke, config};
use stellar_registry_build::registry::Registry;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Entries to batch register, in the format `name:address` or `name:address:owner`.
    /// Can be specified multiple times.
    #[arg(long, required = true)]
    pub entry: Vec<String>,

    /// Registry channel prefix (e.g. "unverified")
    #[arg(long)]
    pub channel: Option<String>,

    /// Prepares and simulates without invoking
    #[arg(long)]
    pub dry_run: bool,

    #[command(flatten)]
    pub config: global::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Registry(#[from] stellar_registry_build::Error),
    #[error("Invalid entry format '{0}'. Expected 'name:address' or 'name:address:owner'")]
    InvalidEntry(String),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let registry = Registry::new(&self.config, self.channel.as_deref()).await?;

        let mut json_entries = String::from("[");
        let default_source = self.config.source_account().await?.to_string();
        let mut parsed: Vec<(String, String, String)> = Vec::new();

        for entry_str in &self.entry {
            let parts: Vec<&str> = entry_str.splitn(3, ':').collect();
            let (name, address, owner) = match parts.len() {
                2 => (
                    parts[0].to_string(),
                    parts[1].to_string(),
                    default_source.clone(),
                ),
                3 => (
                    parts[0].to_string(),
                    parts[1].to_string(),
                    parts[2].to_string(),
                ),
                _ => return Err(Error::InvalidEntry(entry_str.clone())),
            };
            parsed.push((name, address, owner));
        }

        for (i, (name, address, owner)) in parsed.iter().enumerate() {
            if i > 0 {
                json_entries.push(',');
            }
            let _ = write!(json_entries, r#"["{name}","{address}","{owner}"]"#);
        }
        json_entries.push(']');

        let invoke_args = ["batch_register", "--contracts", &json_entries];

        registry
            .as_contract()
            .invoke(&invoke_args, self.dry_run)
            .await?;

        eprintln!(
            "{}Successfully staged {} contracts for batch registration",
            if self.dry_run { "Dry Run: " } else { "" },
            parsed.len()
        );
        Ok(())
    }
}
