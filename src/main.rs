pub mod cli_errors;
use std::sync::Arc;

use clap::Parser;
use dashmap::DashMap;
use owo_colors::OwoColorize;

//
use crate::{
    cli_commands_parser::validate_cli_commands::validate_command,
    cli_errors::CliErrors,
    logs::init_logs::{self, init_logging},
};
pub mod cli_commands_parser;
pub mod docker;

pub mod logs;
pub mod yaml_parser;
pub mod utils;

/**
 * argument can a file_path/network_name
 */
#[derive(Parser, Debug)]
pub struct CLI {
    cli_name: String,
    cli_command: String,
    argument: String,
}

pub struct cli_memory {
    pub service_map: DashMap<String, usize>,
}

#[tokio::main]
async fn main() -> Result<(), CliErrors> {
    // init_logging();

    let image_map = DashMap::<String, usize>::new();

    let app_state = Arc::new(cli_memory {
        service_map: image_map,
    });

    // how to share dashmap between multiple services
    // we need to see that

    // we need to create a sahred hashmap , which we can use from anywhere
    // since multiple threads can try to change the data at once
    // so to make it safe , we will put it in arc
    // plus mutex , to create lock ststem
    // but lock service will stop other all threads to either read or write
    // so better apporach is , rw map , buthere also , also if multiple thread want to write (even if multiple things ) , it will stop
    // so more better version is dashmap
    // it will divide the hasmap in multiple parts , and hece it will stop only those threads which want to change + or add same thing

    let cli = CLI::parse();

    validate_command(
        cli.cli_name,
        cli.cli_command,
        cli.argument,
        Arc::clone(&app_state),
    )
    .await?;

    Ok(())

    // println!("this is the cli name = {:?} , comand name = {:?} , file path = {} " , cli.cli_name , cli.cli_command , cli.file_path);
}
