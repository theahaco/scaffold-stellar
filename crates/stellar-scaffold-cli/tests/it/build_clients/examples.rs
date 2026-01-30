use rstest::rstest;
use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};

#[rstest]
#[case::hello_world("stellar/hello_world")]
#[case::account("stellar/account")]
#[case::alloc("stellar/alloc")]
#[case::atomic_swap("stellar/atomic_swap")]
#[case::auth("stellar/auth")]
#[case::bls_signature("stellar/bls_signature")]
#[case::custom_types("stellar/custom_types")]
#[case::deep_contract_auth("stellar/deep_contract_auth")]
#[case::errors("stellar/errors")]
#[case::eth_abi("stellar/eth_abi")]
#[case::events("stellar/events")]
#[case::fuzzing("stellar/fuzzing")]
#[case::groth16_verifier("stellar/groth16_verifier")]
#[case::import_ark_bn254("stellar/import_ark_bn254")]
#[case::increment("stellar/increment")]
#[case::increment_with_fuzz("stellar/increment_with_fuzz")]
#[case::increment_with_pause("stellar/increment_with_pause")]
#[case::liquidity_pool("stellar/liquidity_pool")]
#[case::logging("stellar/logging")]
#[case::mint_lock("stellar/mint-lock")]
#[case::other_custom_types("stellar/other_custom_types")]
#[case::pause("stellar/pause")]
#[case::simple_account("stellar/simple_account")]
#[case::single_offer("stellar/single_offer")]
#[case::timelock("stellar/timelock")]
#[case::token("stellar/token")]
#[case::ttl("stellar/ttl")]
fn test_adding_and_building_soroban_examples(#[case] input: &str) {
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

// the following are commented out because they currently do not build from a freshly created scaffold project
// #[case::oz_fungible_merkle_airdrop("oz/fungible-merkle-airdrop")] // also needs hex-literal as a workspace dev dependency
// #[case::oz_multisig("oz/multisig")] - doesn't have a cargo.toml in the root of the example
// #[case::oz_upgradeable("oz/upgradeable")] - doesn't have a cargo.toml in the root of the example
// #[case::oz_rwa("oz/rwa")] - error: symbol `__constructor` is already defined
// #[case::oz_sac_admin_generic("oz/sac-admin-generic")] - also needs workspace.dependencies.ed25519-dalek

#[rstest]
#[case::oz_fungible_allowlist("oz/fungible-allowlist")]
#[case::oz_fungible_blocklist("oz/fungible-blocklist")]
#[case::oz_fungible_capped("oz/fungible-capped")]
#[case::oz_fungible_pausable("oz/fungible-pausable")]
#[case::oz_fungible_vault("oz/fungible-vault")]
#[case::oz_merkle_voting("oz/merkle-voting")]
#[case::oz_nft_access_control("oz/nft-access-control")]
#[case::oz_nft_consecutive("oz/nft-consecutive")]
#[case::oz_nft_enumberable("oz/nft-enumerable")]
#[case::oz_nft_royalties("oz/nft-royalties")]
#[case::oz_nft_sequential_minting("oz/nft-sequential-minting")]
#[case::oz_ownable("oz/ownable")]
#[case::oz_pausable("oz/pausable")]
#[case::oz_sac_admin_wrapper("oz/sac-admin-wrapper")]
fn test_adding_and_building_oz_examples(#[case] input: &str) {
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

// this test makes sure that the OZ example repo release version that is included in new scaffold projects from the FE template is compatible with the current scaffold binary
#[tokio::test]
async fn test_scaffold_project_is_compatible_with_oz_examples() {
    TestEnv::from_init("test-project", |env| async move {
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

        // generate all of these examples into an existing scaffold project
        for example in vec![
            "oz/fungible-allowlist",
            "oz/fungible-capped",
            "oz/fungible-pausible",
        ] {
            env.scaffold("generate")
                .arg("contract")
                .arg("--from")
                .arg(example)
                .assert()
                .success()
                .stdout_as_str();
        }

        // Run scaffold_build and assert success
        env.scaffold_build("development", true).assert().success();
    })
    .await;
}
