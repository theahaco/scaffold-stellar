use std::{convert::Infallible, str::FromStr};

use stellar_cli::{commands::contract::invoke, config};

use crate::{contract::ContractId, registry::Registry};

#[derive(Clone, Debug)]
/// Help docs for special type
pub struct PrefixedName {
    pub channel: Option<String>,
    pub name: String,
}

impl FromStr for PrefixedName {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((channel, name)) = s.split_once("/") {
            Ok(Self {
                channel: Some(channel.to_owned()),
                name: name.to_owned(),
            })
        } else {
            Ok(Self {
                channel: None,
                name: s.to_owned(),
            })
        }
    }
}

impl From<PrefixedName> for ContractId {
    fn from(value: PrefixedName) -> Self {
        Self::FromRegistry(value)
    }
}

impl PrefixedName {
    pub async fn registry(&self, config: &config::Args) -> Result<Registry, invoke::Error> {
        Registry::from_named_registry(config, &self).await
    }
}
