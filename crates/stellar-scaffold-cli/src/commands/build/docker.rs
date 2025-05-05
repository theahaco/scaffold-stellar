use stellar_cli::{commands as cli, CommandParser};
use std::error::Error;

pub async fn start_local_stellar() -> Result<(), Box<dyn Error>> {
    let result = cli::container::StartCmd::parse_arg_vec(&["local"])?
        .run(&stellar_cli::commands::global::Args::default())
        .await;
    if let Err(e) = result {
        if e.to_string().contains("already in use")
            || e.to_string().contains("port is already allocated")
        {
            eprintln!("Container is already running, proceeding to health check...");
        } else {
            return Err(Box::new(e));
        }
    }
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    wait_for_stellar_health().await
}
async fn wait_for_stellar_health() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(60);
    loop {
        let elapsed_time = start_time.elapsed();
        if elapsed_time > timeout {
            eprintln!("Timeout reached: stopping health checks.");
            return Err("Health check timed out".into());
        }
        let res = client
            .post("http://localhost:8000/rpc")
            .header("Content-Type", "application/json")
            .body(r#"{"jsonrpc": "2.0", "id": 1, "method": "getHealth"}"#)
            .send()
            .await?;
        if res.status().is_success() {
            let health_status: serde_json::Value = res.json().await?;
            if health_status["result"]["status"] == "healthy" {
                break;
            }
            eprintln!("Stellar status is not healthy: {health_status:?}");
        } else {
            eprintln!("Health check request failed with status: {}", res.status());
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        eprintln!("Retrying health check.");
    }
    Ok(())
}
