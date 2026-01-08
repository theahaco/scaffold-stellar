use clap::Parser;
use stellar_cli::{commands::global, print::Print};

/// A command to clean the scaffold-generated artifacts from a project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {}

#[derive(thiserror::Error, Debug)]
pub enum Error {}

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        printer.infoln("Starting workspace cleanup");
        todo!();
    }
}
