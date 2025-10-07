use crate::{error::Error, name::canonicalize, Contract, ContractClient as SorobanContractClient};
use assert_matches::assert_matches;
use soroban_sdk::{
    self,
    testutils::{Address as _, BytesN as _, MockAuth, MockAuthInvoke},
    vec, Address, Bytes, BytesN, Env, IntoVal,
};
extern crate std;

fn default_version(env: &Env) -> soroban_sdk::String {
    soroban_sdk::String::from_str(&env, "0.0.0")
}

stellar_registry::import_contract_client!(registry);
// Equivalent to:

// mod registry {
//     use super::soroban_sdk;
//     soroban_sdk::contractimport!(file = "../../../../target/stellar/registry.wasm");
// }

fn to_string(env: &Env, s: &str) -> soroban_sdk::String {
    soroban_sdk::String::from_str(env, s)
}

fn init() -> Registry<'static> {
    let e = Env::default();
    let env = &e.clone();
    let admin = Address::generate(env);
    let client = SorobanContractClient::new(env, &env.register(Contract, (admin.clone(),)));
    Registry {
        env: env.clone(),
        client,
        admin,
    }
}

struct Registry<'a> {
    pub env: Env,
    pub client: SorobanContractClient<'a>,
    pub admin: Address,
}

impl<'a> Registry<'a> {
    fn try_publish(&self, author: &Address) -> Result<(), Error> {
        let bytes = self.bytes();
        let version = default_version(self.env());
        match self
            .client
            .try_publish(&self.name(), author, &bytes, &version)
        {
            Ok(_) => Ok(()),
            Err(e) => {
                std::println!("Publish error: {:#?}", e);
                Err(e.unwrap())
            }
        }
    }

    // fn publish_with_author(&self, author: &Address) {
    //     let bytes = self.bytes();
    //     let version = default_version(self.env());
    //     self.client.publish(&self.name(), author, &bytes, &version);
    // }

    fn publish(&self) {
        self.try_publish(&self.admin).unwrap()
    }
    fn env(&self) -> &Env {
        &self.env
    }

    fn name(&self) -> soroban_sdk::String {
        soroban_sdk::String::from_str(self.env(), "registry")
    }

    fn bytes(&self) -> Bytes {
        Bytes::from_slice(self.env(), registry::WASM)
    }

    fn hash(&self) -> BytesN<32> {
        self.env().deployer().upload_contract_wasm(registry::WASM)
    }

    fn mock_publish(
        &self,
        wasm_name: &soroban_sdk::String,
        author: &Address,
        version: &Option<soroban_sdk::String>,
        bytes: &Bytes,
    ) {
        let env = self.env();
        env.mock_auths(&[MockAuth {
            address: &author,
            invoke: &MockAuthInvoke {
                contract: &self.client.address,
                fn_name: "publish",
                args: vec![
                    env,
                    wasm_name.into_val(env),
                    author.into_val(env),
                    bytes.into_val(env),
                    version.clone().into_val(env),
                ],
                sub_invokes: &[],
            },
        }]);
    }
}

// fn publish_registry(
//     env: &Env,
//     client: &SorobanContractClient<'static>,
//     author: &Address,
// ) -> Registry {
//     let (r, error) = try_publish_registry(env, client, author);
//     error.unwrap();
//     r
// }

// fn try_publish_registry(
//     env: &Env,
//     client: &SorobanContractClient<'static>,
//     author: &Address,
// ) -> (Registry, Option<Error>) {
//     let registry = Registry::new(env);
//     let bytes = registry.bytes();
//     let version = default_version();
//     let res = client.try_publish(&registry.name(), author, &bytes, &version);
//     std::println!("Publish result: {:#?}", res);
//     (registry, res.err().map(|e| e.unwrap()))
// }

#[test]
fn wasm_error_cases() {
    let registry = &init();
    let env = registry.env();
    let name = &registry.name();
    assert_matches!(
        registry.client.try_fetch_hash(name, &None).unwrap_err(),
        Ok(Error::NoSuchWasmPublished)
    );
    assert_matches!(
        registry.client.try_fetch_hash(name, &None).unwrap_err(),
        Ok(Error::NoSuchWasmPublished)
    );
    env.mock_all_auths();
    registry.publish();
    assert_eq!(registry.client.fetch_hash(name, &None), registry.hash());
    assert_matches!(
        registry
            .client
            .try_fetch_hash(name, &Some(to_string(env, "0.0.1")))
            .unwrap_err(),
        Ok(Error::NoSuchVersion)
    );

    // let other_address = Address::generate(env);
    // let res = client
    //     .try_publish(name, &other_address, &bytes, &None, &None)
    //     .unwrap_err();

    // assert!(matches!(res, Ok(Error::AlreadyPublished)));
}

#[test]
fn contract_error_cases() {
    let registry = &init();
    let env = registry.env();

    let name = &to_string(env, "contract");
    let wasm_name = &registry.name();
    assert_matches!(
        registry.client.try_fetch_contract_id(name).unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );
    let author = &registry.admin;
    env.mock_all_auths();
    registry.publish();
    registry.client.deploy(
        wasm_name,
        &None,
        name,
        author,
        &Some(vec![env, author.into_val(env)]),
    );

    assert_matches!(
        registry
            .client
            .try_deploy(
                wasm_name,
                &None,
                name,
                author,
                &Some(vec![env, author.into_val(env)])
            )
            .unwrap_err(),
        Ok(Error::AlreadyDeployed)
    );
}

#[test]
fn contract_admin_error_cases() {
    let registry = &init();
    let env = &registry.env().clone();
    let _ = Address::generate(env);
    let other_address = Address::generate(env);

    let name = &to_string(env, "registry");
    let wasm_name = &registry.name();
    let author = &registry.admin;
    assert_matches!(
        registry.client.try_fetch_contract_id(name).unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );
    let version = &Some(default_version(env));
    registry.mock_publish(name, &other_address, version, &registry.bytes());
    // env.mock_auths(&[MockAuth {
    //     address: &other_address,
    //     invoke: &MockAuthInvoke {
    //         contract: &registry.client.address,
    //         fn_name: "publish",
    //         args: vec![
    //             env,
    //             wasm_name.into_val(env),
    //             other_address.into_val(env),
    //             registry.bytes().into_val(env),
    //             .into_val(env),
    //         ],
    //         sub_invokes: &[],
    //     },
    // }]);

    // &self.name(), author, &bytes, &version)
    // registry.client.publish(
    //     wasm_name,
    //     &other_address,
    //     &registry.bytes(),
    //     &default_version(),
    // );
    assert_eq!(
        registry.try_publish(&other_address).unwrap_err(),
        Error::AdminOnly
    );
    env.mock_auths(&[MockAuth {
        address: author,
        invoke: &MockAuthInvoke {
            contract: &registry.client.address,
            fn_name: "publish",
            args: vec![
                env,
                wasm_name.into_val(env),
                author.into_val(env),
                registry.bytes().into_val(env),
                default_version(env).into_val(env),
            ],
            sub_invokes: &[],
        },
    }]);
    registry.publish();
    let author_val: soroban_sdk::Val = other_address.into_val(env);
    env.mock_auths(&[MockAuth {
        address: author,
        invoke: &MockAuthInvoke {
            contract: &registry.client.address,
            fn_name: "deploy",
            args: vec![
                env,
                wasm_name.clone().into_val(registry.env()),
                ().into_val(env),
                other_address.into_val(env),
                vec![env, author_val].into_val(env),
            ],
            sub_invokes: &[],
        },
    }]);
    assert_eq!(
        registry.client.try_deploy(
            wasm_name,
            &None,
            name,
            &other_address,
            &Some(vec![env, other_address.into_val(env)]),
        ),
        Err(Ok(Error::AdminOnly))
    );

    // assert_matches!(
    //     registry
    //         .client
    //         .try_deploy(
    //             wasm_name,
    //             &None,
    //             name,
    //             author,
    //             &Some(vec![author.into_val(env)])
    //         )
    //         .unwrap_err(),
    //     Ok(Error::AlreadyDeployed)
    // );
}

#[test]
fn returns_most_recent_version() {
    let registry = init();
    let client = &registry.client;
    let env = registry.env();
    let name = &registry.name();
    env.mock_all_auths();
    let address = &registry.admin;
    registry.publish();
    let fetched_hash = client.fetch_hash(name, &None);
    let wasm_hash = registry.hash();
    assert_eq!(fetched_hash, wasm_hash);
    let second_hash: BytesN<32> = BytesN::random(&env);
    client.publish_hash(name, address, &second_hash, &to_string(&env, "0.0.1"));
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);

    assert!(client
        .try_publish_hash(name, address, &second_hash, &to_string(&env, "0.0.2"),)
        .is_err());

    let second_hash: BytesN<32> = BytesN::random(&env);
    client.publish_hash(name, address, &second_hash, &to_string(&env, "0.0.9"));
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);
    let second_hash: BytesN<32> = BytesN::random(&env);
    client.publish_hash(name, address, &second_hash, &to_string(&env, "0.0.10"));

    let version = client.current_version(name);
    assert_eq!(version, to_string(&env, "0.0.10"));
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);
}

#[test]
fn validate_names() {
    fn test_string(s: &str, result: bool) {
        assert!(
            canonicalize(&to_string(&Env::default(), s)).is_ok() == result,
            "{s} should be {}valid",
            if result { "" } else { "in" }
        );
    }

    test_string("publish", true);
    test_string("a_a_b", true);
    test_string("abcdefghabcdefgh", true);
    test_string("abcdefghabcdefghabcdefghabcdefgh", true);
    test_string(
        "abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh",
        true,
    );
    test_string(
        "abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgha",
        false,
    );
    test_string("a-a_b", true);
    test_string("a-a]]]_b", false);
    test_string("_ab", false);
    test_string("-ab", false);
    test_string("1ab", false);

    assert_eq!(
        canonicalize(&to_string(&Env::default(), "ls_test")).unwrap(),
        to_string(&Env::default(), "ls-test")
    );
    assert_eq!(
        canonicalize(&to_string(&Env::default(), "ls-test")).unwrap(),
        to_string(&Env::default(), "ls-test")
    );

    assert_eq!(
        canonicalize(&to_string(&Env::default(), "Test")).unwrap(),
        to_string(&Env::default(), "test")
    );
    assert_eq!(
        canonicalize(&to_string(&Env::default(), "Ls-teSt")).unwrap(),
        to_string(&Env::default(), "ls-test")
    );
}

#[test]
fn publish_to_kebab_case() {
    let registry = &init();
    let client = &registry.client;
    let address = &registry.admin;
    let env = registry.env();
    let name = &to_string(&env, "hello_world");
    // client.register_name(address, name);
    let bytes = Bytes::from_slice(&env, registry::WASM);
    env.mock_all_auths();
    let version = default_version(&env);
    client.publish(name, address, &bytes, &version);
    let most_recent_version = client.current_version(&to_string(&env, "hello_world"));
    assert_eq!(most_recent_version, to_string(&env, "0.0.0"));
}

#[test]
fn validate_version() {
    let registry = &init();
    let client = &registry.client;
    let address = &registry.admin;
    let env = registry.env();
    let name = &to_string(&env, "registry");
    let bytes = &Bytes::from_slice(&env, registry::WASM);
    env.mock_all_auths();
    let version = &to_string(&env, "0.0.0");
    let new_version = &to_string(&env, "0.0.1");
    client.publish(name, address, bytes, version);
    let random_hash: BytesN<32> = BytesN::random(&env);
    assert_eq!(
        client.try_publish_hash(name, address, &random_hash, version),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );
    assert_eq!(
        client.try_publish_hash(name, address, &random_hash, &to_string(&env, "0.  0.0"),),
        Err(Ok(Error::InvalidVersion))
    );
    client.publish_hash(name, address, &random_hash, new_version);
    assert_eq!(
        client.try_publish_hash(name, address, &BytesN::<32>::random(&env), version),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );
}
