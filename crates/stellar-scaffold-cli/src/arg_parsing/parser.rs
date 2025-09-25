#![allow(clippy::result_large_err)]
use dialoguer::{Confirm, Input, Select};

use super::Error;

pub struct ArgParser {
    skip_prompt: bool,
}

impl ArgParser {
    pub fn get_constructor_args(
        skip_prompt: bool,
        contract_name: &str,
        wasm: &[u8],
    ) -> Result<Option<String>, Error> {
        let entries = soroban_spec_tools::contract::Spec::new(wasm)?.spec;
        let spec = soroban_spec_tools::Spec::new(&entries);

        // Check if constructor function exists
        let Ok(func) = spec.find_function("__constructor") else {
            return Ok(None);
        };
        if func.inputs.is_empty() {
            return Ok(None);
        }

        // Build the custom command for the constructor
        let cmd = super::build_custom_cmd("__constructor", &spec)?;
        let parser = Self { skip_prompt };

        println!("\n📋 Contract '{contract_name}' requires constructor arguments:");
        let args = cmd
            .get_arguments()
            .filter(|arg| !arg.get_id().as_str().ends_with("-file-path"))
            .filter_map(|arg| parser.handle_constructor_argument(arg).transpose())
            .collect::<Result<Vec<_>, _>>()?
            .join(" ");
        Ok((!args.is_empty()).then_some(args))
    }

    fn handle_constructor_argument(&self, arg: &clap::Arg) -> Result<Option<String>, Error> {
        let arg_name = arg.get_id().as_str();

        let help_text = arg.get_long_help().or(arg.get_help()).map_or_else(
            || "No description available".to_string(),
            ToString::to_string,
        );

        let value_name = arg
            .get_value_names()
            .map_or_else(|| "VALUE".to_string(), |names| names.join(" "));

        // Display help text before the prompt
        println!("\n  --{arg_name}");
        if value_name != "bool" && !help_text.is_empty() {
            println!("   {help_text}");
        }

        if value_name == "bool" {
            self.handle_bool_argument(arg_name)
        } else if value_name.contains('|') && is_simple_enum(&value_name) {
            self.handle_simple_enum_argument(arg_name, &value_name)
        } else {
            // For all other types (complex enums, structs, strings), use string input
            self.handle_formatted_input(arg_name, true).map(Some)
        }
    }

    fn handle_formatted_input(&self, arg_name: &str, with_name: bool) -> Result<String, Error> {
        let input_result: String = if self.skip_prompt {
            String::new()
        } else {
            Input::new()
                .with_prompt(format!("Enter value for --{arg_name}"))
                .allow_empty(true)
                .interact()?
        };

        let value = input_result.trim();

        let value = if value.is_empty() {
            "# TODO: <Fill in value>"
        } else {
            // Check if the value is already quoted
            let is_already_quoted = (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''));

            // Only wrap in quotes if it's not already quoted and contains special characters or spaces
            if !is_already_quoted
                && (value.contains(' ')
                    || value.contains('{')
                    || value.contains('[')
                    || value.contains('"'))
            {
                &format!("'{value}'")
            } else {
                value
            }
        };
        if with_name {
            Ok(format!("--{arg_name} {value}"))
        } else {
            Ok(value.to_string())
        }
    }

    fn handle_simple_enum_argument(
        &self,
        arg_name: &str,
        value_name: &str,
    ) -> Result<Option<String>, Error> {
        // Parse the values from "a | b | c" format and add numeric options
        let values: Vec<_> = value_name.split('|').collect();

        if self.skip_prompt {
            return Ok(Some(format!(
                "--{arg_name} TODO: Pick One <{}>",
                values.join(" | ")
            )));
        }

        let mut select = Select::new()
            .with_prompt(format!("Select value for --{arg_name}"))
            .default(0); // This will show the cursor on the first option initially

        // Add "Skip" option
        select = select.item("(Skip - leave blank)");

        for value in &values {
            select = select.item(format!("Value: {value}"));
        }

        let selection = select.interact()?;

        Ok((selection > 0).then(|| {
            // User selected an actual value (not skip)
            let selected_value = values[selection - 1];
            format!("--{arg_name} {selected_value}")
        }))
    }

    fn handle_bool_argument(&self, arg_name: &str) -> Result<Option<String>, Error> {
        if self.skip_prompt {
            return Ok(Some(format!("TODO add or remove <--{arg_name}>")));
        }
        let bool_value = Confirm::new()
            .with_prompt(format!("Set --{arg_name} to true?"))
            .default(false)
            .interact()?;
        Ok(bool_value.then(|| format!("--{arg_name}")))
    }

    pub fn get_upgrade_args(contract_name: &str) -> Result<String, Error> {
        println!("\n📋 Contract '{contract_name}' requires upgrade arguments:");
        let parser = Self { skip_prompt: false };
        parser.handle_formatted_input("operator", false)
    }
}

fn is_simple_enum(value_name: &str) -> bool {
    value_name.split('|').all(|part| {
        let trimmed = part.trim();
        trimmed.parse::<i32>().is_ok() || trimmed.chars().all(|c| c.is_alphabetic() || c == '_')
    })
}
