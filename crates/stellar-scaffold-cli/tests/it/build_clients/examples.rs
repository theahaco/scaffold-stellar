use rstest::rstest;
use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};

#[rstest]
#[case::hello_world("hello_world")]
#[case::account("account")]
#[case::alloc("alloc")]
#[case::atomic_swap("atomic_swap")]
#[case::auth("auth")]
#[case::bls_signature("bls_signature")]
#[case::custom_types("custom_types")]
#[case::deep_contract_auth("deep_contract_auth")]
#[case::errors("errors")]
#[case::eth_abi("eth_abi")]
#[case::events("events")]
#[case::fuzzing("fuzzing")]
#[case::groth16_verifier("groth16_verifier")]
#[case::import_ark_bn254("import_ark_bn254")]
#[case::increment("increment")]
#[case::increment_with_fuzz("increment_with_fuzz")]
#[case::increment_with_pause("increment_with_pause")]
#[case::liquidity_pool("liquidity_pool")]
#[case::logging("logging")]
#[case::mint_lock("mint-lock")]
#[case::other_custom_types("other_custom_types")]
#[case::pause("pause")]
#[case::simple_account("simple_account")]
#[case::single_offer("single_offer")]
#[case::timelock("timelock")]
#[case::token("token")]
#[case::ttl("ttl")]
fn test_adding_and_building_example_work(#[case] input: &str) {
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
            .arg(input)
            .arg("--output")
            .arg(format!("{}/contracts/example", env.cwd.display()))
            .assert()
            .success()
            .stdout_as_str();

        // Run scaffold_build and assert success
        env.scaffold_build("development", true).assert().success();
    });
}
