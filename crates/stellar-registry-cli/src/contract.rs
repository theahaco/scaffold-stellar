use sha2::{Digest, Sha256};

use soroban_rpc as rpc;
use stellar_cli::{
    commands::{NetworkRunnable, contract::invoke},
    config::{self, UnresolvedContract, network::Network},
    xdr::{self, WriteXdr},
};

pub const REGISTRY_NAME: &str = "registry";

pub trait NetworkContract {
    fn contract_id(&self) -> Result<stellar_strkey::Contract, config::Error>;

    fn contract_sc_address(&self) -> Result<xdr::ScAddress, config::Error> {
        Ok(xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash(
            self.contract_id()?.0,
        ))))
    }

    fn invoke_registry(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> impl std::future::Future<Output = Result<String, invoke::Error>> + Send;

    fn view_registry(
        &self,
        slop: &[&str],
    ) -> impl std::future::Future<Output = Result<String, invoke::Error>> + Send;

    fn rpc_client(&self) -> Result<rpc::Client, config::Error>;
}

impl NetworkContract for config::Args {
    fn contract_id(&self) -> Result<stellar_strkey::Contract, config::Error> {
        let Network {
            network_passphrase, ..
        } = &self.get_network()?;
        let contract: UnresolvedContract = unsafe {
            if let Ok(contract_id) = std::env::var("STELLAR_REGISTRY_CONTRACT_ID") {
                contract_id.parse()
            } else {
                contract_id(network_passphrase).to_string().parse()
            }
            .unwrap_unchecked()
        };
        Ok(contract.resolve_contract_id(&self.locator, network_passphrase)?)
    }

    async fn invoke_registry(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> Result<String, invoke::Error> {
        invoke_registry(slop, self, fee, view_only).await
    }

    async fn view_registry(&self, slop: &[&str]) -> Result<String, invoke::Error> {
        invoke_registry(slop, self, None, true).await
    }

    fn rpc_client(&self) -> Result<rpc::Client, config::Error> {
        Ok(rpc::Client::new(&self.get_network()?.rpc_url)?)
    }
}

pub fn build_invoke_cmd(
    slop: &[&str],
    config: &stellar_cli::config::Args,
    fee: Option<&stellar_cli::fee::Args>,
    view_only: bool,
) -> Result<invoke::Cmd, config::Error> {
    Ok(invoke::Cmd {
        contract_id: UnresolvedContract::Resolved(config.contract_id()?),
        slop: slop.iter().map(Into::into).collect(),
        config: config.clone(),
        fee: fee.cloned().unwrap_or_default(),
        send: if view_only {
            invoke::Send::No
        } else {
            invoke::Send::Default
        },
        ..Default::default()
    })
}

pub async fn invoke_registry(
    slop: &[&str],
    config: &stellar_cli::config::Args,
    fee: Option<&stellar_cli::fee::Args>,
    view_only: bool,
) -> Result<String, invoke::Error> {
    Ok(build_invoke_cmd(slop, config, fee, view_only)?
        .run_against_rpc_server(
            Some(&stellar_cli::commands::global::Args::default()),
            Some(config),
        )
        .await?
        .into_result()
        .expect("Failed to parse JSON"))
}

pub fn stellar_address() -> stellar_strkey::ed25519::PublicKey {
    "GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M"
        .parse()
        .unwrap()
}

pub fn contract_id(network_passphrase: &str) -> stellar_strkey::Contract {
    let network_id = xdr::Hash(Sha256::digest(network_passphrase.as_bytes()).into());
    let preimage = xdr::HashIdPreimage::ContractId(xdr::HashIdPreimageContractId {
        network_id,
        contract_id_preimage: xdr::ContractIdPreimage::Address(
            xdr::ContractIdPreimageFromAddress {
                address: xdr::ScAddress::Account(xdr::AccountId(
                    xdr::PublicKey::PublicKeyTypeEd25519(stellar_address().0.into()),
                )),
                salt: xdr::Uint256([0; 32]),
            },
        ),
    });
    let preimage_xdr = preimage
        .to_xdr(xdr::Limits::none())
        .expect("HashIdPreimage should not fail encoding to xdr");
    stellar_strkey::Contract(Sha256::digest(preimage_xdr).into())
}

#[cfg(test)]
mod generate_id {
    use stellar_cli::config::network::passphrase::*;

    fn test_contract_id((passphrase, contract_id): (&str, &str)) {
        assert_eq!(
            &super::contract_id(passphrase).to_string(),
            contract_id,
            "{passphrase}"
        );
    }
    #[test]
    fn futurenet() {
        test_contract_id((
            FUTURENET,
            "CACPZCQSLEGF6QOSBF42X6LOUQXQB2EJRDKNKQO6US6ZZH5FD6EB325M",
        ));
    }

    #[test]
    fn testnet() {
        test_contract_id((
            TESTNET,
            "CBCOGWBDGBFWR5LQFKRQUPFIG6OLOON35PBKUPB6C542DFZI3OMBOGHX",
        ));
    }

    #[test]
    fn mainnet() {
        test_contract_id((
            MAINNET,
            "CC3SILHAJ5O75KMSJ5J6I5HV753OTPWEVMZUYHS4QEM2ZTISQRAOMMF4",
        ));
    }

    #[test]
    fn local() {
        test_contract_id((
            LOCAL,
            "CCMHAZ6QTUUF2W4PUBW5BAI6R75BVKIUVHJU6IBQTWCS5RBASDOKHF7T",
        ));
    }
}
