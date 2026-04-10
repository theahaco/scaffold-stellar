use assert_cmd::cargo::cargo_bin;
use stellar_scaffold_test::{TestEnv, rpc_url};

/// Returns a PATH string with the reporter binary's directory prepended so
/// `which stellar-scaffold-reporter` finds it during extension discovery.
fn reporter_path() -> String {
    let reporter_bin = cargo_bin("stellar-scaffold-reporter");
    format!(
        "{}:{}",
        reporter_bin.parent().unwrap().display(),
        TestEnv::stellar_path()
    )
}

#[test]
fn reporter_standard_output() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        let log_path = "target/scaffold-reporter/build.log";

        env.set_environments_toml(format!(
            r#"
development.accounts = [
    {{ name = "alice" }},
]
development.extensions = ["reporter"]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false

[development.ext.reporter]
mode = "standard"
log_file = "{log_path}"
"#,
            rpc_url()
        ));

        let output = env
            .scaffold_build("development", false)
            .env("PATH", reporter_path())
            .output()
            .expect("Failed to run scaffold build");

        assert!(
            output.status.success(),
            "Build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        // post-compile: timing and WASM sizes table
        assert!(stdout.contains("📋 Compile time:"), "compile time missing");
        assert!(
            stdout.contains("📋 WASM sizes:"),
            "WASM sizes header missing"
        );
        assert!(
            stdout.contains("soroban_hello_world_contract:"),
            "WASM size entry missing"
        );

        // post-deploy: contract details including fresh deploy kind
        assert!(
            stdout.contains("📋 Deployed soroban_hello_world_contract (deployed fresh):"),
            "deploy details missing"
        );
        assert!(stdout.contains("    id = "), "contract id missing");
        assert!(stdout.contains("    hash = "), "wasm hash missing");

        // post-codegen: timing and package size
        assert!(
            stdout.contains("📋 Codegen soroban_hello_world_contract:"),
            "codegen details missing"
        );
        assert!(
            stdout.contains("    package size ="),
            "package size missing"
        );

        // post-dev: build cycle summary
        assert!(
            stdout.contains("📋 build cycle complete:"),
            "post-dev summary missing"
        );

        // log file should exist and contain the same output
        let log_file = env.cwd.join(log_path);
        assert!(log_file.exists(), "log file was not created");
        let log_content = std::fs::read_to_string(log_file).unwrap();
        assert!(log_content.contains("📋 Compile time:"));
        assert!(log_content.contains("📋 build cycle complete:"));
    });
}

#[test]
fn reporter_minimal_output() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(format!(
            r#"
development.accounts = [
    {{ name = "alice" }},
]
development.extensions = ["reporter"]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false

[development.ext.reporter]
mode = "minimal"
warn_size_kb = 0.1
"#,
            rpc_url()
        ));

        let output = env
            .scaffold_build("development", false)
            .env("PATH", reporter_path())
            .output()
            .expect("Failed to run scaffold build");

        assert!(
            output.status.success(),
            "Build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        // post-dev summary always fires
        assert!(
            stdout.contains("📋 build cycle complete:"),
            "post-dev summary missing"
        );

        // per-contract metrics suppressed at minimal verbosity
        assert!(
            !stdout.contains("📋 Compile time:"),
            "compile time should be suppressed at minimal"
        );
        assert!(
            !stdout.contains("📋 WASM sizes:"),
            "WASM sizes should be suppressed at minimal"
        );
        assert!(
            !stdout.contains("📋 Deployed"),
            "deploy details should be suppressed at minimal"
        );
        assert!(
            !stdout.contains("📋 Codegen"),
            "codegen details should be suppressed at minimal"
        );

        // warn_size_kb fires regardless of verbosity level
        assert!(
            stdout.contains("⚠️") && stdout.contains("exceeds threshold of"),
            "WASM size warning should fire at any verbosity"
        );
    });
}
