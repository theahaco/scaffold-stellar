use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::BTreeMap as Map;
use std::path::Path;
use toml::value::Table;

use crate::commands::build::clients::ScaffoldEnv;

pub const ENV_FILE: &str = "environments.toml";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("⛔ ️parsing environments.toml: {0}")]
    ParsingToml(#[from] toml::de::Error),
    #[error("⛔ ️no settings for current STELLAR_SCAFFOLD_ENV ({0:?}) found in environments.toml")]
    NoSettingsForCurrentEnv(String),
    #[error("⛔ ️reading environments.toml as a string: {0}")]
    ParsingString(#[from] std::io::Error),
}

type Environments = Map<Box<str>, Environment>;

#[derive(Debug, Clone)]
pub struct Environment {
    pub accounts: Option<Vec<Account>>,
    pub network: Network,
    pub contracts: Option<IndexMap<Box<str>, Contract>>,
}

fn deserialize_accounts<'de, D>(deserializer: D) -> Result<Option<Vec<Account>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<Vec<AccountRepresentation>> = Option::deserialize(deserializer)?;
    Ok(opt.map(|vec| vec.into_iter().map(Account::from).collect()))
}

impl<'de> Deserialize<'de> for Environment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EnvironmentHelper {
            #[serde(default, deserialize_with = "deserialize_accounts")]
            accounts: Option<Vec<Account>>,
            network: Network,
            contracts: Option<Table>,
        }

        let helper = EnvironmentHelper::deserialize(deserializer)?;

        let contracts = helper
            .contracts
            .map(|contracts_table| {
                contracts_table
                    .into_iter()
                    .map(|(key, value)| {
                        let contract: Contract =
                            Contract::deserialize(value).map_err(serde::de::Error::custom)?;
                        Ok((key.into_boxed_str(), contract))
                    })
                    .collect::<Result<IndexMap<_, _>, D::Error>>()
            })
            .transpose()?;

        Ok(Environment {
            accounts: helper.accounts,
            network: helper.network,
            contracts,
        })
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Network {
    pub name: Option<String>,
    pub rpc_url: Option<String>,
    pub network_passphrase: Option<String>,
    pub rpc_headers: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub run_locally: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum AccountRepresentation {
    Simple(String),
    Detailed(Account),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(default)]
    pub default: bool,
}

impl From<AccountRepresentation> for Account {
    fn from(rep: AccountRepresentation) -> Self {
        match rep {
            AccountRepresentation::Simple(name) => Account {
                name,
                default: false,
            },
            AccountRepresentation::Detailed(account) => account,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Contract {
    #[serde(default = "default_client", skip_serializing_if = "std::ops::Not::not")]
    pub client: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_deploy: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constructor_args: Option<String>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            client: default_client(),
            after_deploy: None,
            id: None,
            constructor_args: None,
        }
    }
}

fn default_client() -> bool {
    true
}

impl Environment {
    pub fn get(
        workspace_root: &Path,
        scaffold_env: &ScaffoldEnv,
    ) -> Result<Option<Environment>, Error> {
        let env_toml = workspace_root.join(ENV_FILE);

        if !env_toml.exists() {
            return Ok(None);
        }

        let toml_str = std::fs::read_to_string(env_toml)?;
        let mut parsed_toml: Environments = toml::from_str(&toml_str)?;
        let current_env = parsed_toml.remove(scaffold_env.to_string().as_str());
        if current_env.is_none() {
            return Err(Error::NoSettingsForCurrentEnv(scaffold_env.to_string()));
        }
        Ok(current_env)
    }
}

impl From<&Network> for stellar_cli::config::network::Args {
    fn from(network: &Network) -> Self {
        stellar_cli::config::network::Args {
            network: network.name.clone(),
            rpc_url: network.rpc_url.clone(),
            network_passphrase: network.network_passphrase.clone(),
            rpc_headers: network.rpc_headers.clone().unwrap_or_default(),
        }
    }
}
