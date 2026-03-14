// if this file we will recievr the command terms , and we will validate that wheter its a valid cli commadn or not , if not we will show error

use std::{env, fmt, fs, path::Path, str::FromStr, sync::Arc};

use docker_compose_types::Compose;

use crate::{
    cli_errors::CliErrors,
    cli_memory,
    docker::{container_logs::container_logs, delete_container::delete_container},
    yaml_parser::yaml_parser,
};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum CliName {
    DockYard,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CliCommands {
    Up,
    Down,
    Logs,
    Status,
}

impl fmt::Display for CliName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// from string functiong to covert a string to respective enum value , if exist
impl FromStr for CliCommands {
    type Err = ();
    fn from_str(input: &str) -> Result<CliCommands, Self::Err> {
        match input {
            "Up" => Ok(CliCommands::Up),
            "Down" => Ok(CliCommands::Down),
            "Logs" => Ok(CliCommands::Logs),
            "Status" => Ok(CliCommands::Status),
            _ => Err(()),
        }
    }
}

impl fmt::Display for CliCommands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/**
 * we will validate command , if cli name , command type and file path is valid or not
 */
pub async fn validate_command(
    cli_name: String,
    cli_command: String,
    argument: String, //docker compose file path
    app_state: Arc<cli_memory>,
) -> Result<bool, CliErrors> {
    if cli_name != CliName::DockYard.to_string() {
        return Err(CliErrors::wrong_cli_name());
    }

    // converting command to respective enum command , if exist
    let equivalent_command_enum =
        CliCommands::from_str(&cli_command).map_err(|_| CliErrors::wrong_cli_command())?;

    match equivalent_command_enum {
        CliCommands::Up => {
            // here argument is the docker compose.yaml file path
            yaml_parser(&argument, app_state).await.map_err(|e| e)?;
        }
        CliCommands::Down => {
            println!("this is the down command process");
            // here the argument is network
            delete_container(&argument).await.map_err(|e| e)?;
        }
        CliCommands::Logs => {
            println!("this is the logs command proces");
            container_logs(argument).await?;
        }
        CliCommands::Status => {
            println!("this is the status command process");
        }
    }

    Ok(true)
}
