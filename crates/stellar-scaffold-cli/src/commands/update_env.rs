#![allow(clippy::struct_excessive_bools)]
use clap::Parser;
use std::{
    fs,
    io::{self, BufRead},
};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// name of env var to update
    #[arg(long)]
    pub name: String,
    /// value of env var to update, if not provided stdin is used
    #[arg(long)]
    pub value: Option<String>,
    /// Path to .env file
    #[arg(long, default_value = ".env")]
    pub env_file: std::path::PathBuf,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let file = &self.env_file;
        let env_file = if file.exists() {
            fs::read_to_string(file)?
        } else {
            String::new()
        };

        let value = self.value.clone().unwrap_or_else(|| {
            // read from stdin
            std::io::stdin()
                .lock()
                .lines()
                .next()
                .expect("stdin closed")
                .expect("stdin error")
        });
        let name = &self.name;
        let new_env_file =
            replace_lines_starting_with(&env_file, &format!("{name}="), &format!("{name}={value}"));
        fs::write(&self.env_file, new_env_file)?;
        Ok(())
    }
}

fn replace_lines_starting_with(input: &str, starts_with: &str, replacement: &str) -> String {
    let mut found = false;
    let mut v = input
        .lines()
        .map(|line| {
            if line.starts_with(starts_with) {
                found = true;
                replacement
            } else {
                line
            }
        })
        .collect::<Vec<&str>>();
    if !found {
        v.push(replacement);
    }
    v.join("\n")
}
