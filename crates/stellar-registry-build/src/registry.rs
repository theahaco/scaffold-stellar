use stellar_cli::config;

use crate::{
    Error,
    contract::{Contract, PreHashContractID},
    named_registry::PrefixedName,
};

pub struct Registry(Contract);

impl Registry {
    pub async fn from_named_registry(
        config: &config::Args,
        name: &PrefixedName,
    ) -> Result<Self, Error> {
        Self::new(config, name.channel.as_deref()).await
    }
    pub async fn new(config: &config::Args, name: Option<&str>) -> Result<Self, Error> {
        let contract = Self::verified(config)?;
        Ok(if let Some(name) = name {
            if let Ok(contract_id) = name.parse() {
                Registry(Contract::new(contract_id, config))
            } else {
                contract.fetch_contract(name).await.map(Registry)?
            }
        } else {
            contract
        })
    }

    pub async fn fetch_contract_id(&self, name: &str) -> Result<stellar_strkey::Contract, Error> {
        let slop = ["fetch_contract_id", "--contract-name", name];
        let contract_id = self.0.invoke_with_result(&slop, true).await?;
        contract_id
            .trim_matches('"')
            .parse()
            .map_err(|_| Error::InvalidContractId(contract_id))
    }

    pub async fn fetch_contract(&self, name: &str) -> Result<Contract, Error> {
        Ok(Contract::new(
            self.fetch_contract_id(name).await?,
            self.0.config(),
        ))
    }

    pub fn as_contract(&self) -> &Contract {
        &self.0
    }

    pub fn verified(config: &config::Args) -> Result<Self, Error> {
        Ok(Registry(Contract::new(
            if let Ok(id) = std::env::var("STELLAR_REGISTRY_CONTRACT_ID") {
                id.parse().map_err(|_| Error::InvalidContractId(id))?
            } else {
                verified_contract_id(&config.get_network()?.network_passphrase)
            },
            config,
        )))
    }
}

/// Stellar Address for G account for registry project
/// # Unsafe
/// It parse
pub fn stellar_address() -> stellar_strkey::ed25519::PublicKey {
    unsafe {
        "GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M"
            .parse()
            .unwrap_unchecked()
    }
}

pub fn contract_id(network_passphrase: &str, salt: &str) -> stellar_strkey::Contract {
    PreHashContractID::new(stellar_address(), salt)
        .id(&stellar_build::Network::from_passphrase(network_passphrase).unwrap())
}

pub fn verified_contract_id(network_passphrase: &str) -> stellar_strkey::Contract {
    contract_id(network_passphrase, "v0.5.0")
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
            "CDMAKNALA4EKEA52CP645Y6H5NUM5AZPOPBM5RHOG2SRNHUOAPFHK6P4",
        ));
    }

    #[test]
    fn testnet() {
        test_contract_id((
            TESTNET,
            "CCA256DWBJJEEYXAWQHP5N4ZAJ2NW4P5T52LZCGC766Q5XHFVNQBMFZV",
        ));
    }

    #[test]
    fn mainnet() {
        test_contract_id((
            MAINNET,
            "CAYVNQYGQ7IVZBBKMZ46UNRUQIFGBVHVZFCG47CYCMA2SAODDVDVCWMS",
        ));
    }

    #[test]
    fn local() {
        test_contract_id((
            LOCAL,
            "CB7GPZFAAJQJYJD63P7HUAVABBSGLRWJB2C35RKR5TQ33AMRSS2XFL3C",
        ));
    }
}
