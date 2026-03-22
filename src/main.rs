#![allow(warnings)]

pub mod cli_errors;
use std::sync::Arc;

use clap::Parser;
use dashmap::DashMap;
use owo_colors::OwoColorize;
use tokio::{signal, sync::Mutex};

use crate::{
    cli_commands_parser::validate_cli_commands::validate_command,
    cli_errors::CliErrors,
    docker::stop_container::{self, stop_container},
    logs::init_logs::{self, init_logging},
    yaml_parser::{FilePathType, file_name},
};
pub mod cli_commands_parser;
pub mod docker;

pub mod logs;
pub mod utils;
pub mod yaml_parser;

/**
 * argument can a file_path/network_name
 */
#[derive(Parser, Debug)]
pub struct CLI {
    cli_name: String,
    cli_command: String,
    argument: String,
}

pub struct CliMemory {
    pub service_map: DashMap<String, usize>,
    pub current_network: Mutex<Option<String>>,
}

#[tokio::main]
async fn main() -> Result<(), CliErrors> {
    let image_map = DashMap::<String, usize>::new();

    let mut app_state = Arc::new(CliMemory {
        service_map: image_map,
        current_network: Mutex::new(None), // now known at this time
    });

    // tokio is running multiple service paralleley
    // app is ruuning , as well as detecting ctrl+c is also parallely running
    tokio::select! {

        _ = run_app(Arc::clone(&app_state)) =>{},
        _ = signal::ctrl_c() =>{
            println!("ctrl +c is detected , stopping all containers of current running process network and label");
            cleanup(Arc::clone(&app_state)).await;

        }

    }

    Ok(())
}

/**
 * whole cli app
 */
pub async fn run_app(app_state: Arc<CliMemory>) -> Result<(), CliErrors> {
    let cli = CLI::parse();

    // validate the cli command and check argument and do the task
    validate_command(
        cli.cli_name,
        cli.cli_command,
        cli.argument,
        Arc::clone(&app_state),
    )
    .await?;

    Ok(())
}

/**
 * if ctrl +c is clicked anywhere in between the process of running app , tokio::signal::ctrl_c() will detect it and call this function
 * and we will stop all the container
 *
 * mainly it is for when we are starting/restarting the conatiners
 * else where it does not affect that much
 */
pub async fn cleanup(app_state: Arc<CliMemory>) -> Result<(), CliErrors> {
    match app_state.current_network.lock().await.clone() {
        Some(network_name) => {
            stop_container(&network_name.clone()).await;
        }
        None => {
            // network name is not present , so we do not do anything
        }
    };

    Ok(())
}
