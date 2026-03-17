use bollard::{
    Docker,
    container::LogOutput,
    query_parameters::{LogsOptions, LogsOptionsBuilder},
};
use futures_util::StreamExt;

use crate::{
    cli_errors::CliErrors,
    logs::service_logs::{general_error_message, general_message},
};


/**
 * it will take cont name ir id and show all the logs till now of the cont
 */
pub async fn container_logs(cont_name: String) -> Result<bool, CliErrors> {
    println!("i came here");

    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    let options = LogsOptionsBuilder::default().stdout(true).build();

    let opt = LogsOptions {
        follow: false,
        stdout: true,
        stderr: true,
        timestamps: false,
        tail: String::from("all"),
        ..Default::default()
    };
    let mut a = docker.logs(&cont_name, Some(opt));

    general_message(&cont_name, "conatiner logs : ");
    while let Some(log_data) = a.next().await {

        // then for tommrow we will see the pointers and improve the codebase and add diffculty

        let res = log_data.map_err(|e| CliErrors::new(e.to_string()))?;

        match res {
            LogOutput::StdOut { message } => {
                println!("{}", String::from_utf8_lossy(&message));
            }
            // showing error in red , it it occurs
            LogOutput::StdErr { message } => {
                println!("{}", String::from_utf8_lossy(&message));
            }
            LogOutput::Console { message } => {
                print!("{}", String::from_utf8_lossy(&message));
            }

            _ => {}
        }
    }
    Ok(true)
}
