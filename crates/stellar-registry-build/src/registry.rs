use stellar_cli::{commands::contract::invoke, config};

use crate::{
    contract::{Contract, PreHashContractID},
    named_registry::PrefixedName,
};

pub struct Registry(Contract);

impl Registry {
    pub async fn from_named_registry(
        config: &config::Args,
        name: &PrefixedName,
    ) -> Result<Self, invoke::Error> {
        Self::new(config, name.channel.as_deref()).await
    }
    pub async fn new(config: &config::Args, name: Option<&str>) -> Result<Self, invoke::Error> {
        let id = verified_contract_id(&config.get_network()?.network_passphrase);
        let contract = Registry(Contract::new(id, config));
        if let Some(name) = name {
            if let Ok(contract_id) = name.parse() {
                Ok(Registry(Contract::new(contract_id, config)))
            } else {
                contract.fetch_contract(name).await.map(Registry)
            }
        } else {
            Ok(contract)
        }
    }

    pub async fn fetch_contract_id(
        &self,
        name: &str,
    ) -> Result<stellar_strkey::Contract, invoke::Error> {
        let slop = ["fetch_contract_id", "--contract-name", name];
        let contract_id = self.0.invoke_with_result(&slop, None, true).await?;
        Ok(contract_id.trim_matches('"').parse().unwrap())
    }

    pub async fn fetch_contract(&self, name: &str) -> Result<Contract, invoke::Error> {
        Ok(Contract::new(
            self.fetch_contract_id(name).await?,
            self.0.config(),
        ))
    }

    pub fn as_contract(&self) -> &Contract {
        &self.0
    }
}

pub fn stellar_address() -> stellar_strkey::ed25519::PublicKey {
    "GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M"
        .parse()
        .unwrap()
}

pub fn contract_id(network_passphrase: &str, salt: &str) -> stellar_strkey::Contract {
    PreHashContractID::new(stellar_address(), salt).id(&network_passphrase.parse().unwrap())
}

pub fn verified_contract_id(network_passphrase: &str) -> stellar_strkey::Contract {
    contract_id(network_passphrase, "verified")
}

pub fn unverified_contract_id(network_passphrase: &str) -> stellar_strkey::Contract {
    contract_id(network_passphrase, "unverified")
}

#[cfg(test)]
mod generate_id {
    use stellar_cli::config::network::passphrase::*;

    fn test_contract_id((passphrase, contract_id): (&str, &str)) {
        assert_eq!(
            &super::verified_contract_id(passphrase).to_string(),
            contract_id,
            "{passphrase}"
        );
    }
    #[test]
    fn futurenet() {
        test_contract_id((
            FUTURENET,
            "CBUP2U7IY4GBZWILAGFGBOGEJEVSWZ6FAIKAX2L7PYOEE7R556LNXRJM",
        ));
    }

    #[test]
    fn testnet() {
        test_contract_id((
            TESTNET,
            "CBFFTTX7QKA76FS4LHHQG54BC7JF5RMEX4RTNNJ5KEL76LYHVO3E3OEE",
        ));
    }

    #[test]
    fn mainnet() {
        test_contract_id((
            MAINNET,
            "CCRKU6NT4CRG4TVKLCCJFU7EOSAUBHWGBJF2JWZJSKTJTXCXXTKOJIUS",
        ));
    }

    #[test]
    fn local() {
        test_contract_id((
            LOCAL,
            "CDUK4O7FPAPZWAMS6PBKM7E4IO5MCBJ2ZPZ6K2GOHK33YW7Q4H7YZ35Z",
        ));
    }
}
