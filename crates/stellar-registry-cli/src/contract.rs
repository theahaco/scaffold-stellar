use sha2::{Digest, Sha256};

use soroban_rpc as rpc;
use stellar_cli::{
    commands::{contract::invoke, NetworkRunnable},
    config::{self, network::Network, UnresolvedContract},
    xdr::{self, WriteXdr},
};

pub trait NetworkContract {
    fn contract_id(&self) -> Result<stellar_strkey::Contract, config::Error>;

    fn contract_sc_address(&self) -> Result<xdr::ScAddress, config::Error> {
        Ok(xdr::ScAddress::Contract(xdr::Hash(self.contract_id()?.0)))
    }

    fn invoke_registry(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
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
    ) -> Result<String, invoke::Error> {
        invoke_registry(slop, self, fee).await
    }

    fn rpc_client(&self) -> Result<rpc::Client, config::Error> {
        Ok(rpc::Client::new(&self.get_network()?.rpc_url)?)
    }
}

pub fn build_invoke_cmd(
    slop: &[&str],
    config: &stellar_cli::config::Args,
    fee: Option<&stellar_cli::fee::Args>,
) -> Result<invoke::Cmd, config::Error> {
    Ok(invoke::Cmd {
        contract_id: UnresolvedContract::Resolved(config.contract_id()?),
        slop: slop.iter().map(Into::into).collect(),
        config: config.clone(),
        fee: fee.cloned().unwrap_or_default(),
        ..Default::default()
    })
}

pub async fn invoke_registry(
    slop: &[&str],
    config: &stellar_cli::config::Args,
    fee: Option<&stellar_cli::fee::Args>,
) -> Result<String, invoke::Error> {
    Ok(build_invoke_cmd(slop, config, fee)?
        .run_against_rpc_server(Some(&stellar_cli::commands::global::Args::default()), None)
        .await?
        .into_result()
        .expect("Failed to parse JSON"))
}

pub fn stellar_address() -> stellar_strkey::ed25519::PublicKey {
    "GBLTLNPISIK2JFDN42MXET7K7QLFSTPBAL5FLK7QUH2VM5HTCURFQGDK"
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
mod tests {
    #[test]
    fn test_contract_id() {
        let contract_id = super::contract_id("Test SDF Future Network ; October 2022");
        assert_eq!(
            contract_id.to_string(),
            "CBBL2SVHBQY35LY6IE64RHS5K2M7BXXV72KXS2CKV6MU4L3Y33HANUJ2".to_string()
        );
    }
}
