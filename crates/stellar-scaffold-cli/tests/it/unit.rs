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
    let project_name = format!(
        "my-project-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let project_path = env.cwd.join(&project_name);
    if project_path.exists() {
        std::fs::remove_dir_all(&project_path).unwrap();
    }
    assert!(!project_path.exists());
    env.scaffold("init")
        .args([project_path.to_str().unwrap()])
        .assert()
        .success();
    // Verify frontend template files exist
    assert!(project_path.join("package.json").exists());
    assert!(project_path.join("src").exists());
    assert!(project_path.join("tsconfig.json").exists());
}

#[tokio::test]
async fn clean_removes_scaffold_artifacts() {
    let env = TestEnv::new("soroban-init-boilerplate");

    // ensure we have a target/stellar dir before running clean
    env.scaffold("build").assert().success();
    let target_stellar = env.cwd.join("target").join("stellar");
    assert!(target_stellar.exists(), "target/stellar should exist");

    // when cleaning from current dir
    env.scaffold("clean").assert().success();

    // verify target/stellar is removed
    assert!(!target_stellar.exists(), "target/stellar should be removed");

    // Verify output contains expected messages
    // assert!(stderr.contains("Cleaning scaffold artifacts"));
    // assert!(stderr.contains("Clean complete"));

    // Verify target/stellar is removed
    // assert!(!target_stellar.exists(), "target/stellar should be removed");

    // // Verify generated package is removed
    // assert!(!test_package.exists(), "generated package should be removed");

    // // Verify .gitkeep is preserved
    // assert!(packages_dir.join(".gitkeep").exists(), ".gitkeep should be preserved");

    // // Verify generated file in src/contracts is removed
    // assert!(!src_contracts_dir.join("generated.ts").exists(), "generated.ts should be removed");

    // // Verify git-tracked file is preserved
    // assert!(src_contracts_dir.join("util.ts").exists(), "util.ts should be preserved");
}
