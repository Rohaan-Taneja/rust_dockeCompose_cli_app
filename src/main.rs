pub mod cli_errors;
use clap::Parser;

use crate::{cli_commands_parser::validate_cli_commands::validate_command, cli_errors::CliErrors };
pub mod cli_commands_parser;

pub mod yaml_parser;

#[derive(Parser , Debug)]
pub struct CLI {
    cli_name : String ,
    cli_command : String ,
    file_path : String


}

#[tokio::main]
async fn main() -> Result<() , CliErrors> {
    println!("Hello, world!");

    let cli = CLI::parse();

    validate_command(cli.cli_name, cli.cli_command, cli.file_path).await.map_err(|e| e)?;

    Ok(())

    // println!("this is the cli name = {:?} , comand name = {:?} , file path = {} " , cli.cli_name , cli.cli_command , cli.file_path);
}
