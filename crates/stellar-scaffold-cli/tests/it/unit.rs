use crate::util::{AssertExt, TestEnv};

#[test]
fn contract_with_bad_name_prints_useful_error() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(
            r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
hello.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
        );

        env.scaffold("build")
            .assert()
            .failure()
            .stderr(predicates::str::contains("No contract named \"hello\""));
    });
}

#[test]
fn no_environments_toml_ends_after_contract_build() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        assert!(
            stderr.contains("Finished"),
            "expected the 'Finished' message, got: {stderr}"
        );
    });
}

#[test]
fn uses_manifest_path_for_build_command() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(
            r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
hello_world.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
        );

        let stderr = env
            .scaffold("build")
            .current_dir(env.cwd.join(".."))
            .args(["--manifest-path", "./soroban-init-boilerplate/Cargo.toml"])
            .assert()
            .success()
            .stderr_as_str();

        assert!(stderr.contains("🌐 using network at http://localhost:8000/rpc\n"));
    });
}
