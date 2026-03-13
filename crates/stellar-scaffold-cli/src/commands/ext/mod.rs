use clap::Parser;

pub mod ls;

#[derive(Parser, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// List extensions configured for the active environment, showing their
    /// version, resolution status, and supported hooks.
    Ls(ls::Cmd),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Ls(#[from] ls::Error),
}
