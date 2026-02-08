use stellar_cli::{
    commands::contract::invoke,
    config::{self, locator},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid contract id: {0}")]
    InvalidContractId(String),
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Locator(#[from] locator::Error),
    #[error(transparent)]
    Build(#[from] stellar_build::networks::Error),
}
