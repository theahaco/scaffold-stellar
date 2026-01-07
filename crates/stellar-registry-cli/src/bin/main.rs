use clap::{CommandFactory, Parser};

use stellar_cli::config::Config;
use stellar_registry_cli::Root;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv().unwrap_or_default();
    set_env_from_config();
    let mut root = Root::try_parse().unwrap_or_else(|e| {
        let mut cmd = Root::command();
        e.format(&mut cmd).exit();
    });

    if let Err(e) = root.run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

// Load ~/.config/stellar/config.toml defaults as env vars.
fn set_env_from_config() {
    if let Ok(config) = Config::new() {
        set_env_value_from_config("STELLAR_ACCOUNT", config.defaults.identity);
        set_env_value_from_config("STELLAR_NETWORK", config.defaults.network);
    }
}

// Set an env var from a config file if the env var is not already set.
// Additionally, a `$NAME_SOURCE` variant will be set, which allows
// `stellar env` to properly identity the source.
fn set_env_value_from_config(name: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };

    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var(format!("{name}_SOURCE")) };

    if std::env::var(name).is_err() {
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(name, value) };
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(format!("{name}_SOURCE"), "use") };
    }
}
