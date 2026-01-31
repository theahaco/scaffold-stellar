use sha2::{Digest, Sha256};
use std::str::FromStr;

#[derive(Default, Debug)]
pub enum Network {
    #[default]
    Local,
    Testnet,
    Futurenet,
    Mainnet,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid STELLAR_NETWORK: {0}. Expected: local, testnet, futurenet, or mainnet.")]
    InvalidNetwork(String),
    #[error(
        r#"Invalid STELLAR_PASSPHRASE: {0}. 
            Expected: "Standalone Network ; February 2017",
                      "Test SDF Network ; September 2015",
                      "Test SDF Future Network ; October 2022",
                      "Public Global Stellar Network ; September 2015" "#
    )]
    InvalidNetworkPassphrase(String),
}

impl FromStr for Network {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Network::Local),
            "testnet" => Ok(Network::Testnet),
            "futurenet" => Ok(Network::Futurenet),
            "mainnet" => Ok(Network::Mainnet),
            other => Err(Error::InvalidNetwork(other.to_string())),
        }
    }
}

impl Network {
    pub fn from_env() -> Self {
        std::env::var("STELLAR_NETWORK")
            .as_deref()
            .unwrap_or("local")
            .parse()
            .unwrap_or_default()
    }

    pub fn passphrase_from_env() -> Self {
        std::env::var("STELLAR_NETWORK_PASSPHRASE")
            .ok()
            .and_then(|s| Self::from_passphrase(&s).ok())
            .unwrap_or_else(Self::from_env)
    }

    pub fn from_passphrase(passphrase: &str) -> Result<Self, Error> {
        Ok(match passphrase {
            "Standalone Network ; February 2017" => Network::Local,
            "Test SDF Network ; September 2015" => Network::Testnet,
            "Test SDF Future Network ; October 2022" => Network::Futurenet,
            "Public Global Stellar Network ; September 2015" => Network::Mainnet,
            other => return Err(Error::InvalidNetworkPassphrase(other.to_string())),
        })
    }

    pub fn passphrase(&self) -> &str {
        match self {
            Network::Local => "Standalone Network ; February 2017",
            Network::Testnet => "Test SDF Network ; September 2015",
            Network::Futurenet => "Test SDF Future Network ; October 2022",
            Network::Mainnet => "Public Global Stellar Network ; September 2015",
        }
    }

    /// Returns the network ID as a hash
    pub fn id(&self) -> [u8; 32] {
        Sha256::digest(self.passphrase().as_bytes()).into()
    }
}
