extern crate std;
use crate::{error::Error, Contract, ContractClient as SorobanContractClient};

use soroban_sdk::{
    self,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Bytes, BytesN, Env, TryIntoVal, Val, Vec,
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
}

impl<'a> Registry<'a> {
    pub fn new() -> Self {
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
        Bytes::from_slice(self.env(), registry::WASM)
    }

    pub fn hash(&self) -> BytesN<32> {
        self.env().deployer().upload_contract_wasm(registry::WASM)
    }

    pub fn mock_initial_publish(&self) {
        let env = self.env();
        let name = self.name();
        let author = self.admin();
        let version = default_version(env);
        let bytes = self.bytes();
        self.mock_publish(&name, author, &Some(version), &bytes);
    }

    pub fn mock_publish(
        &self,
        wasm_name: &soroban_sdk::String,
        author: &Address,
        version: &Option<soroban_sdk::String>,
        bytes: &Bytes,
    ) {
        self.mock_method(author, "publish", (wasm_name, author, bytes, version));
    }

    pub fn mock_method(
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
}
