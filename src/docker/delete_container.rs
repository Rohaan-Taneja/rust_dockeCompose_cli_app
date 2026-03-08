use std::{collections::HashMap, default};

use bollard::{
    Docker,
    query_parameters::{
        InspectNetworkOptions, ListContainersOptions, ListContainersOptionsBuilder,
        RemoveContainerOptionsBuilder,
    },
    secret::ContainerSummary,
};

use crate::cli_errors::CliErrors;

pub async fn delete_container(network_name: &str) -> Result<bool, CliErrors> {
    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    // validating whether use has given a valid existing network id or not
    validate_network(&docker, network_name).await?;

    let docker_cont_list = list_all_filter_conatiners(&docker, "network", network_name).await?;

    // println!("this is the list of cont {:?}", docker_cont_list);
    for cont in docker_cont_list {
        let cont_id = cont
            .id
            .ok_or_else(|| CliErrors::new("cannot find container id".to_string()))?;
        let image_name = cont
            .image
            .ok_or_else(|| CliErrors::new("cannot find container id".to_string()))?;
        println!(
            "this is the list of conatiners in this network {} {}",
            cont_id, image_name
        );

        let options = RemoveContainerOptionsBuilder::default().force(true).build();
        docker.remove_container(&cont_id, Some(options)).await.map_err(|e| CliErrors::new(e.to_string()))?;
    
        println!("stopped the cont id => {}" , cont_id);
    }

    // removing the network after deleting all the containers
    docker.remove_network(network_name).await.map_err(|e| CliErrors::new(e.to_string()))?;

    Ok(true)
}

/**
 * if we get specifc netwrok_name => details , it means the network is present , else it will show error
 */
pub async fn validate_network(docker: &Docker, network_name: &str) -> Result<(), CliErrors> {
    docker
        .inspect_network(network_name, None)
        .await
        .map_err(|e| CliErrors::new(e.to_string()))?;

    Ok(())
}

/**
 * function to get list of all the containers with specific filters
 */
pub async fn list_all_filter_conatiners(
    docker: &Docker,
    filter_name: &str,
    filter_val: &str,
) -> Result<Vec<ContainerSummary>, CliErrors> {
    let mut filters = HashMap::<String, Vec<String>>::new();

    if !filter_name.is_empty() && !filter_val.is_empty() {
        filters.insert(filter_name.to_owned(), vec![filter_val.to_owned()]);
    }

    let options = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();

    let docker_cont_list = docker
        .list_containers(Some(options))
        .await
        .map_err(|e| CliErrors::new("getting errorin listing containers".to_string()))?;

    Ok(docker_cont_list)
}
