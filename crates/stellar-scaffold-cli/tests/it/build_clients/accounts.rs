use std::fs;
use stellar_scaffold_test::{AssertExt, TestEnv};
#[test]
fn create_two_accounts() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(r#"
[development]
network = { rpc-url = "http://localhost:8000/rpc", network-passphrase = "Standalone Network ; February 2017"}

accounts = [
    "alice",
    { name = "bob" },
]
[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#);
        for dir in fs::read_dir(&env.cwd).unwrap() {
            println!("Found directory: {:?}", dir);
        }

        let stderr = env
            .scaffold_build("development", true)
            .assert()
            .success()
            .stderr_as_str();
        println!("{stderr}");
        assert!(stderr.contains("Creating keys for \"alice\""));
        assert!(stderr.contains("Creating keys for \"bob\""));

        // check that they dont get overwritten if build is run again
        let stderr = env
            .scaffold_build("development", true)
            .assert()
            .success()
            .stderr_as_str();
        assert!(stderr.contains("identity with the name \'alice\' already exists"));
        assert!(stderr.contains("identity with the name \'bob\' already exists"));
        for dir in fs::read_dir(&env.cwd.join(".stellar")).unwrap() {
            println!("Found directory: {:?}", dir);
        }

        // check that they're actually funded
        let stderr = env
            .stellar("keys")
            .args([
                "fund",
                "alice",
                "--network-passphrase",
                "\"Standalone Network ; February 2017\"",
                "--rpc-url",
                "http://localhost:8000/soroban/rpc",
            ])
            .assert()
            .success()
            .stderr_as_str();
        assert!(stderr.contains("Account AliasOrSecret(\"alice\") funded"));
    });
}

#[test]
fn funding_existing_account_toml() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(r#"
[development]
network = { rpc-url = "http://localhost:8000/rpc", network-passphrase = "Standalone Network ; February 2017"}

accounts = [
    "alice",
]
[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#);

        // Create alice.toml manually, simulating a pre-existing identity
        env.stellar("keys")
            .args([
                "generate",
                "alice",
                "--network-passphrase",
                "\"Standalone Network ; February 2017\"",
                "--rpc-url",
                "http://localhost:8000/soroban/rpc",
            ])
            .assert()
            .success();

        // Run scaffold_build and assert success
        env.scaffold_build("development", true).assert().success();
    });
}
