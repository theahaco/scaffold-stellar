use crate::util::TestEnv;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;

#[ignore]
#[tokio::test]
async fn dev_command_watches_for_changes_and_environments_toml() {
    TestEnv::from_async("soroban-init-boilerplate", |env| async {
        Box::pin(async move {
            let mut dev_process = env
                .stellar_scaffold_process("dev", &["--build-clients"])
                .current_dir(&env.cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn dev process");

            let stderr = dev_process.stderr.take().unwrap();
            let mut stderr_lines = LinesStream::new(BufReader::new(stderr).lines());

            // Wait for initial build to complete
            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 1: Modify a source file
            let file_changed = "contracts/hello_world/src/lib.rs";
            env.modify_file(file_changed, "// This is a test modification");
            let file_changed_path = env.cwd.join(file_changed);

            // Wait for the dev process to detect changes and rebuild
            TestEnv::wait_for_output(
                &mut stderr_lines,
                &format!("File changed: {file_changed_path:?}"),
            )
            .await;

            TestEnv::wait_for_output(&mut stderr_lines, "cargo rustc").await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 2: Create and modify environments.toml
            env.set_environments_toml(
                r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            );

            // Wait for the dev process to detect changes and rebuild
            TestEnv::wait_for_output(
                &mut stderr_lines,
                "🌐 using network at http://localhost:8000/rpc",
            )
            .await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 3: modify the network url in environments.toml
            env.set_environments_toml(
                r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:9000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            );

            // Wait for the dev process to detect changes and rebuild
            TestEnv::wait_for_output(
                &mut stderr_lines,
                "🌐 using network at http://localhost:9000/rpc",
            )
            .await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 4: remove environments.toml
            let file_changed = "environments.toml";
            env.delete_file(file_changed);

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            dev_process
                .kill()
                .await
                .expect("Failed to kill dev process");
        })
        .await;
    })
    .await;
}

#[tokio::test]
async fn dev_command_start_local_stellar_with_run_locally() {
    TestEnv::from_async("soroban-init-boilerplate", |env| async {
        Box::pin(async move {
            // Set environments.toml with run_locally enabled
            env.set_environments_toml(
                r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"
run-locally = true

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            );

            let mut dev_process = env
                .stellar_scaffold_process("dev", &["--build-clients"])
                .current_dir(&env.cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn dev process");

            let stderr = dev_process.stderr.take().unwrap();
            let mut stderr_lines = LinesStream::new(BufReader::new(stderr).lines());

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Starting local Stellar Docker container...",
            )
            .await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Local Stellar network is healthy and running.",
            )
            .await;

            dev_process
                .kill()
                .await
                .expect("Failed to kill dev process");
        })
        .await;
    })
    .await;
}
