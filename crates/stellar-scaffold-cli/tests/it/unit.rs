use stellar_scaffold_test::{AssertExt, TestEnv};

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
            .stellar_scaffold_custom_dir(
                "build",
                &["--manifest-path", "./soroban-init-boilerplate/Cargo.toml"],
                &env.cwd.join(".."),
            )
            .assert()
            .success()
            .stderr_as_str();

        assert!(stderr.contains("Build Complete"));
    });
}

#[test]
fn init_copies_frontend_template() {
    let env = TestEnv::new_empty();

    // Use a unique project name to avoid pre-existing directory issue
    let project_name = format!("my-project-{}", std::time::SystemTime::now().elapsed().unwrap().as_nanos());
    let project_path = env.cwd.join(&project_name);
    env.scaffold("init")
        .args([project_path.to_str().unwrap()])
        .assert()
        .success();
    // Verify frontend template files exist
    assert!(project_path.join("package.json").exists());
    assert!(project_path.join("src").exists());
    assert!(project_path.join("tsconfig.json").exists());
}
