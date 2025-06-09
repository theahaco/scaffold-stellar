use clap::Parser;

pub mod contract;

#[derive(Parser, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Generate a new contract from examples or wizard
    Contract(Box<contract::Cmd>),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Contract(#[from] contract::Error),
}
