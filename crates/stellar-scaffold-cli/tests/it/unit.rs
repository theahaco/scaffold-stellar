use stellar_scaffold_test::{AssertExt, TestEnv};
use std::fs;

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

#[test]
fn clean_removes_generated_artifacts() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        // Create some test directories and files that should be cleaned
        let target_stellar = env.cwd.join("target").join("stellar");
        let packages_dir = env.cwd.join("packages");
        let src_contracts_dir = env.cwd.join("src").join("contracts");

        // Create target/stellar/local directory with a test file
        fs::create_dir_all(&target_stellar.join("local")).unwrap();
        fs::write(target_stellar.join("local").join("test.wasm"), "test").unwrap();

        // Create a generated package in packages/
        let test_package = packages_dir.join("test-package");
        fs::create_dir_all(&test_package).unwrap();
        fs::write(test_package.join("index.ts"), "export {}").unwrap();

        // Create a generated file in src/contracts/
        fs::write(src_contracts_dir.join("generated.ts"), "export {}").unwrap();

        // Verify files exist before clean
        assert!(target_stellar.exists());
        assert!(test_package.exists());
        assert!(src_contracts_dir.join("generated.ts").exists());
        assert!(src_contracts_dir.join("util.ts").exists()); // git-tracked file should exist

        // Run clean command
        let stderr = env.scaffold("clean").assert().success().stderr_as_str();

        // Verify output contains expected messages
        assert!(stderr.contains("Cleaning scaffold artifacts"));
        assert!(stderr.contains("Clean complete"));

        // Verify target/stellar is removed
        assert!(!target_stellar.exists(), "target/stellar should be removed");

        // Verify generated package is removed
        assert!(!test_package.exists(), "generated package should be removed");

        // Verify .gitkeep is preserved
        assert!(packages_dir.join(".gitkeep").exists(), ".gitkeep should be preserved");

        // Verify generated file in src/contracts is removed
        assert!(!src_contracts_dir.join("generated.ts").exists(), "generated.ts should be removed");

        // Verify git-tracked file is preserved
        assert!(src_contracts_dir.join("util.ts").exists(), "util.ts should be preserved");
    });
}

#[test]
fn clean_handles_environments_toml() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        // Create an environments.toml file
        env.set_environments_toml(
            r#"
[development]
accounts = [
    { name = "alice" },
    { name = "bob" }
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
hello_world.client = false
"#,
        );

        // Run clean command - it should not fail even if stellar keys or alias commands fail
        let result = env.scaffold("clean").assert().success();
        let stderr = result.stderr_as_str();

        // Verify command completed
        assert!(stderr.contains("Cleaning scaffold artifacts"));
        assert!(stderr.contains("Clean complete"));
    });
}


