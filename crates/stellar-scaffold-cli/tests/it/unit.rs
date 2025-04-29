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

#[test]
fn init_copies_contracts_and_frontend_template() {
    let env = TestEnv::new_empty();

    // Run loam init with project path
    let project_path = env.cwd.join("my-project");
    env.stellar("init")
        .args([project_path.to_str().unwrap()])
        .assert()
        .success();
    // Verify contract files exist
    assert!(project_path.join("contracts/core/src/lib.rs").exists());
    assert!(project_path
        .join("contracts/status_message/src/lib.rs")
        .exists());
    assert!(project_path.join("contracts/core/Cargo.toml").exists());
    assert!(project_path
        .join("contracts/status_message/Cargo.toml")
        .exists());

    // Verify frontend template files exist
    assert!(project_path.join("package.json").exists());
    assert!(project_path.join("src").exists());
    assert!(project_path.join("tsconfig.json").exists());
}
