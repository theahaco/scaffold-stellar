use clap::{CommandFactory, Parser};

use stellar_registry_cli::Root;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv().unwrap_or_default();
    let mut root = Root::try_parse().unwrap_or_else(|e| {
        let mut cmd = Root::command();
        e.format(&mut cmd).exit();
    });

    if let Err(e) = root.run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
