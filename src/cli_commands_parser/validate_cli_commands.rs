// if this file we will recievr the command terms , and we will validate that wheter its a valid cli commadn or not , if not we will show error

use std::{env, fmt, fs, path::Path, str::FromStr, sync::Arc};

use docker_compose_types::Compose;

use crate::{
    cli_errors::CliErrors,
    CliMemory,
    docker::{
        container_logs::container_logs, container_status::docker_conatiner_status, delete_container::delete_container, start_images_in_container::build_remote_git_repo, stop_container::stop_container
    },
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
    ContainerStatus,
    Stop,
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
            "ContainerStatus" => Ok(CliCommands::ContainerStatus),
            "Stop" => Ok(CliCommands::Stop),
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
    app_state: Arc<CliMemory>,
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
            yaml_parser(&argument, Arc::clone(&app_state)).await.map_err(|e| e)?;
        }
        CliCommands::Down => {
            println!(
                "this is the down command process , to delete all existing conatiners of this network"
            );
            // here the argument is network
            delete_container(&argument).await.map_err(|e| e)?;
        }
        CliCommands::Logs => {
            println!("this is the logs command proces");
            // here the argument is conatiner name/id
            container_logs(argument).await?;

        }
        CliCommands::ContainerStatus => {
            println!("this is the status command process");
            // here the argument is conatiner id/name
            docker_conatiner_status(argument).await?;
        }
        CliCommands::Stop => {
            println!("this is the stop all conatiners command");
            // here the argument is network id/name
            stop_container(&argument).await?;
        }
    }

    Ok(true)
}
