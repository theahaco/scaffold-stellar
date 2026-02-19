use stellar_cli::{CommandParser, commands as cli};

/// Make sure that `start_local_stellar` is idempotent. It shouldn't fail if container
/// is already running, it should skip startup logic and proceed to health checks.
#[tokio::test]
async fn start_local_stellar_is_idempotent() {
    // Test harness already sets up network, so this is _second_ call
    let result = stellar_scaffold_cli::commands::build::docker::start_local_stellar().await;
    assert!(
        result.is_ok(),
        "start_local_stellar should succeed when the container is already running, got: {result:?}"
    );
}

/// Directly invoke the underlying `container start` CLI command and assert that
/// the error message still contains at least one of the substrings we rely on
/// to detect an already-running container.
///
/// see <https://github.com/theahaco/scaffold-stellar/pull/392>
#[tokio::test]
async fn container_start_error_contains_expected_substring() {
    let cmd = cli::container::StartCmd::parse_arg_vec(&["local"])
        .expect("failed to parse container start command");

    let result = cmd
        .run(&stellar_cli::commands::global::Args::default())
        .await;

    // Verify that the error matches what we're expecting
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            msg.contains("already running") || msg.contains("port is already allocated"),
            "Container start error message changed! \
             Expected it to contain \"already running\" or \"port is already allocated\", \
             but got: \"{msg}\""
        );
    }
}

/// Ensure the Stellar RPC endpoint is healthy
#[tokio::test]
async fn stellar_rpc_health_endpoint_is_healthy() {
    let rpc_url = stellar_scaffold_test::rpc_url();
    let client = reqwest::Client::new();

    let res = client
        .post(&rpc_url)
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc": "2.0", "id": 1, "method": "getHealth"}"#)
        .send()
        .await
        .expect("failed to reach RPC health endpoint");

    assert!(
        res.status().is_success(),
        "RPC health endpoint returned non-success status: {}",
        res.status()
    );

    let body: serde_json::Value = res.json().await.expect("failed to parse health response");
    assert_eq!(
        body["result"]["status"], "healthy",
        "RPC reported unhealthy status: {body:?}"
    );
}

/// Ensure the Friendbot endpoint is healthy
#[tokio::test]
async fn friendbot_is_responding() {
    let base_url = stellar_scaffold_test::rpc_url().replace("/rpc", "");
    let friendbot_url = format!(
        "{base_url}/friendbot?addr=GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
    );
    let client = reqwest::Client::new();

    let res = client
        .get(&friendbot_url)
        .send()
        .await
        .expect("failed to reach friendbot endpoint");

    // This will be 400 error since we're testing a fake address, but that still means the
    // service is working as expected
    let status = res.status();
    assert!(
        status.is_success() || status == reqwest::StatusCode::BAD_REQUEST,
        "Friendbot returned unexpected status: {status}"
    );
}

/// Check for race conditions by `start_local_stellar` multiple times in quick succession
#[tokio::test]
async fn start_local_stellar_multiple_times() {
    for i in 0..3 {
        let result = stellar_scaffold_cli::commands::build::docker::start_local_stellar().await;
        assert!(
            result.is_ok(),
            "start_local_stellar failed on iteration {i}: {result:?}"
        );
    }
}
