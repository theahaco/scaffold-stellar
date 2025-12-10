use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};

#[test]
fn test_adding_and_building_hello_world_example_work() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017" }}

accounts = [
    "alice",
]
[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url()
        ));

        println!("starting test in directory {:?}", env.cwd);
        eprintln!("test directory {:?}", env.cwd);

        assert!(
            env.cwd.join("contracts").is_dir(),
            "no contracts directory found"
        );
        fs_extra::dir::remove(env.cwd.join("contracts"))
            .expect("failed to remove contracts directory");

        env.scaffold("generate")
            .arg("contract")
            .arg("--from")
            .arg("hello_world")
            .arg("--output")
            .arg(format!("{}/contracts/example", env.cwd.display()))
            .assert()
            .success()
            .stdout_as_str();

        // Run scaffold_build and assert success
        env.scaffold_build("development", true).assert().success();
    });
}
