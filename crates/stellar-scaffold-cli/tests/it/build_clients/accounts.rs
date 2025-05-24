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

        let stderr = env
            .scaffold_build("development", true)
            .assert()
            .success()
            .stderr_as_str();
        assert!(stderr.contains("creating keys for \"alice\""));
        assert!(stderr.contains("creating keys for \"bob\""));
        assert!(env.cwd.join(".stellar/identity/alice.toml").exists());
        assert!(env.cwd.join(".stellar/identity/bob.toml").exists());

        // check that they dont get overwritten if build is run again
        let stderr = env
            .scaffold_build("development", true)
            .assert()
            .success()
            .stderr_as_str();
        assert!(stderr.contains("identity with the name \'alice\' already exists"));
        assert!(stderr.contains("identity with the name \'bob\' already exists"));

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
