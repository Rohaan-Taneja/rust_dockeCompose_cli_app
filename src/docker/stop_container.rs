use bollard::{
    Docker,
    query_parameters::{StopContainerOptions},
};

use crate::{
    cli_errors::CliErrors,
    docker::delete_container::{list_all_filter_conatiners, validate_network},
    logs::service_logs::{general_message, service_stop_or_delete_message},
};

pub async fn stop_container(network_name: &str) -> Result<bool, CliErrors> {
    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    // validating whether use has given a valid existing network id or not
    validate_network(&docker, network_name).await?;

    let docker_cont_list = list_all_filter_conatiners(&docker, "network", network_name).await?;

    for cont in docker_cont_list {
        let cont_id = cont
            .id
            .ok_or_else(|| CliErrors::new("cannot find container id".to_string()))?;
       
        let options = StopContainerOptions::default();
        docker
            .stop_container(&cont_id, Some(options))
            .await
            .map_err(|e| CliErrors::new(e.to_string()))?;

        service_stop_or_delete_message(&cont_id, "conatiner stopped");
    }

    general_message(&network_name, "stopped all conatiners in the network");

    Ok(true)
}
