use bollard::Docker;

use crate::{cli_errors::CliErrors, logs::service_logs::general_message};

pub async fn delete_network(docker : &Docker , network_name : &str)-> Result<bool , CliErrors>{
     docker
        .remove_network(network_name)
        .await
        .map_err(|e| CliErrors::new(e.to_string()))?;
    
    general_message(&network_name, "Deleted network and disconnected all associated containers");

    Ok(true)

}