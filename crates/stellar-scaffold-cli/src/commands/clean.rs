use clap::Parser;
use std::fs;
use stellar_cli::{commands::global, print::Print};
/// A command to clean the scaffold-generated artifacts from a project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {}

#[derive(thiserror::Error, Debug)]
pub enum Error {}

// cleans up scaffold artifacts
// - target/stellar
// - contract aliases (for local and test)
// - identify aliases (for local and test)
// - packages/* (but not checked-into-git files like .gitkeep)
// - src/contracts/* (but not checked-into-git files like util.ts)
// - what about target/wasm32v1-none/release/guess_the_number.wasm

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        printer.infoln("Starting workspace cleanup");

        let cargo_metadata = cargo_metadata::MetadataCommand::new()
            .no_deps()
            .exec()
            .unwrap();

        // clean target/stellar
        let target_dir = cargo_metadata.target_directory;
        let stellar_dir = target_dir.join("stellar");
        if stellar_dir.exists() {
            fs::remove_dir_all(&stellar_dir).unwrap(); //todo handle unwrap
        } else {
            println!("{stellar_dir} does not exist");
        }

        Ok(())
    }
}
