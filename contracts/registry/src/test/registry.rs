extern crate std;
use crate::{error::Error, Contract, ContractArgs, ContractClient as SorobanContractClient};

use soroban_sdk::{
    self,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    vec, Address, Bytes, BytesN, ConversionError, Env, IntoVal, InvokeError, String, Symbol,
    TryIntoVal, Val, Vec,
};

pub fn default_version(env: &Env) -> soroban_sdk::String {
    soroban_sdk::String::from_str(&env, "0.0.0")
}

stellar_registry::import_contract_client!(registry);
// Equivalent to:

// mod registry {
//     use super::soroban_sdk;
//     soroban_sdk::contractimport!(file = "../../../../target/stellar/registry.wasm");
// }

pub fn to_string(env: &Env, s: &str) -> soroban_sdk::String {
    soroban_sdk::String::from_str(env, s)
}

pub struct Registry<'a> {
    env: Env,
    client: SorobanContractClient<'a>,
    admin: Address,
    bytes: Bytes,
    hash: BytesN<32>,
}

impl<'a> Registry<'a> {
    pub fn new() -> Self {
        let e = Env::default();
        let env = &e.clone();
        let admin = Address::generate(env);
        let client = SorobanContractClient::new(env, &env.register(Contract, (admin.clone(),)));
        let bytes = Bytes::from_slice(env, registry::WASM);
        let hash = env.deployer().upload_contract_wasm(registry::WASM);
        Registry {
            env: env.clone(),
            client,
            admin,
            bytes,
            hash,
        }
    }

    pub fn new_with_bytes(
        bytes: &dyn Fn(&Env) -> Bytes,
        hash: &dyn Fn(&Env) -> BytesN<32>,
    ) -> Self {
        let e = Env::default();
        let env = &e.clone();
        let admin = Address::generate(env);
        let client = SorobanContractClient::new(env, &env.register(Contract, (admin.clone(),)));
        Registry {
            env: env.clone(),
            client,
            admin,
            bytes: bytes(env),
            hash: hash(env),
        }
    }

    pub fn default_version(&self) -> soroban_sdk::String {
        default_version(self.env())
    }

    pub fn admin(&self) -> &Address {
        &self.admin
    }

    pub fn client(&self) -> &SorobanContractClient<'a> {
        &self.client
    }

    pub fn try_publish(&self, author: &Address) -> Result<(), Error> {
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

    pub fn publish(&self) {
        self.try_publish(&self.admin).unwrap()
    }
    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn name(&self) -> soroban_sdk::String {
        soroban_sdk::String::from_str(self.env(), "registry")
    }

    pub fn bytes(&self) -> Bytes {
        self.bytes.clone()
    }

    pub fn hash(&self) -> BytesN<32> {
        self.hash.clone()
    }

    pub fn mock_initial_publish(&self) {
        let env = self.env();
        let name = self.name();
        let author = self.admin();
        let version = default_version(env);
        let bytes = self.bytes();
        self.mock_auth_for_publish(&name, author, &Some(version), &bytes);
    }

    pub fn mock_auth_for_publish(
        &self,
        wasm_name: &soroban_sdk::String,
        author: &Address,
        version: &Option<soroban_sdk::String>,
        bytes: &Bytes,
    ) {
        self.mock_auths_for(
            &[author, self.admin()],
            "publish",
            (wasm_name, author, bytes, version),
        );
    }

    pub fn mock_auths_for(
        &self,
        addresses: &[&Address],
        fn_name: &str,
        args: impl TryIntoVal<Env, Vec<Val>>,
    ) {
        let env = self.env();
        let invoke = MockAuthInvoke {
            contract: &self.client.address,
            fn_name,
            args: unsafe { args.try_into_val(env).unwrap_unchecked() },
            sub_invokes: &[],
        };
        let auths: std::vec::Vec<MockAuth<'_>> = addresses
            .into_iter()
            .map(|address| MockAuth {
                address,
                invoke: &invoke,
            })
            .collect();
        env.mock_auths(&auths);
    }

    pub fn mock_auth_for(
        &self,
        signer_address: &Address,
        method: &str,
        args: impl TryIntoVal<Env, Vec<Val>>,
    ) {
        let env = self.env();
        env.mock_auths(&[MockAuth {
            address: signer_address,
            invoke: &MockAuthInvoke {
                contract: &self.client.address,
                fn_name: method,
                args: unsafe { args.try_into_val(env).unwrap_unchecked() },
                sub_invokes: &[],
            },
        }]);
    }

    pub fn mock_auth_and_deploy(
        &self,
        author: &Address,
        wasm_name: &soroban_sdk::String,
        name: &soroban_sdk::String,
    ) -> Address {
        let env = self.env();
        let client = self.client();

        self.mock_auths_for(
            &[author, self.admin()],
            "deploy",
            ContractArgs::deploy(
                wasm_name,
                &None,
                name,
                author,
                &Some(vec![env, author.into_val(env)]),
            ),
        );

        client.deploy(
            wasm_name,
            &None,
            name,
            author,
            &Some(vec![env, author.into_val(env)]),
        )
    }

    pub fn mock_auth_and_try_deploy(
        &self,
        author: &Address,
        version: &Option<String>,
        wasm_name: &soroban_sdk::String,
        name: &soroban_sdk::String,
        args: &Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    ) -> Result<Result<Address, ConversionError>, Result<Error, InvokeError>> {
        let client = self.client();
        self.mock_auths_for(
            &[author, self.admin()],
            "deploy",
            ContractArgs::deploy(wasm_name, version, name, author, args),
        );
        client.try_deploy(wasm_name, version, name, author, args)
    }

    pub fn mock_auth_and_try_upgrade(
        &self,
        author: &Address,
        contract_name: &soroban_sdk::String,
        wasm_name: &soroban_sdk::String,
        version: &Option<String>,
        upgrade_fn: &Option<&str>,
        old_contract: &Address,
        wasm_hash: &BytesN<32>,
    ) -> Result<Result<Address, ConversionError>, Result<Error, InvokeError>> {
        let client = self.client();

        let env = self.env();

        let fn_name = upgrade_fn.unwrap_or("upgrade");
        let upgrade_fn = &upgrade_fn.map(|x| Symbol::new(env, x));

        let upgrade_contract_args =
            ContractArgs::upgrade_contract(contract_name, wasm_name, version, upgrade_fn);

        let upgrade_args = ContractArgs::upgrade(wasm_hash);

        env.mock_auths(&[MockAuth {
            address: author,
            invoke: &MockAuthInvoke {
                contract: &self.client.address,
                fn_name: "upgrade_contract",
                args: unsafe { upgrade_contract_args.try_into_val(env).unwrap_unchecked() },
                sub_invokes: &[MockAuthInvoke {
                    contract: old_contract,
                    fn_name,
                    args: unsafe { upgrade_args.try_into_val(env).unwrap_unchecked() },
                    sub_invokes: &[],
                }],
            },
        }]);

        client.try_upgrade_contract(contract_name, wasm_name, version, upgrade_fn)
    }

    pub fn mock_auth_and_try_upgrade_dev_deploy(
        &self,
        author: &Address,
        contract_name: &soroban_sdk::String,
        wasm: &soroban_sdk::Bytes,
        wasm_hash: &soroban_sdk::BytesN<32>,
        old_contract: &Address,
    ) -> Result<Result<Address, ConversionError>, Result<Error, InvokeError>> {
        let client = self.client();

        let env = self.env();

        let upgrade_contract_args = ContractArgs::dev_deploy(contract_name, wasm, &None);

        let upgrade_args = ContractArgs::upgrade(wasm_hash);

        env.mock_auths(&[MockAuth {
            address: author,
            invoke: &MockAuthInvoke {
                contract: &self.client.address,
                fn_name: "dev_deploy",
                args: unsafe { upgrade_contract_args.try_into_val(env).unwrap_unchecked() },
                sub_invokes: &[MockAuthInvoke {
                    contract: old_contract,
                    fn_name: "upgrade",
                    args: unsafe { upgrade_args.try_into_val(env).unwrap_unchecked() },
                    sub_invokes: &[],
                }],
            },
        }]);

        client.try_dev_deploy(contract_name, wasm, &None)
    }
}
