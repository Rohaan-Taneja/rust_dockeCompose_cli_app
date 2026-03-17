use bollard::{
    Docker,
    secret::{EndpointSettings, NetworkCreateRequest, NetworkingConfig},
};

use std::{
    collections::HashMap,
    env,
    fs::{self},
    path::{Path, PathBuf},
    sync::Arc,
};

use docker_compose_types::{Compose, Healthcheck};

use crate::{
    cli_errors::CliErrors,
    cli_memory,
    docker::{
        compose_parser::{self, DockerImageDetails, construct_docker_image_details_map},
        start_images_in_container::{
            build_current_folder_image, build_or_pull_start_image_in_conatiner,
            check_and_start_network_containers, check_image_locally, pull_image_locally,
            start_image_in_container,
        },
    },
};

pub const FILE_NAMES: [&str; 6] = [
    "compose.yaml",
    "compose.yml",
    "docker-compose.yaml",
    "docker-compose.yml",
    "docker-compose.yml",
    "docker-compose.override.yml",
];

pub enum FilePathType {
    CurrentDir,
    FilePath,
}

/**
 * this will tell what do we have to inspect
 * status =>just to check if the conatiner is running or not
 * health => or to chec is conatiner service has stated or not
 */
pub enum ContainerInspectType {
    Status,
    Health(Healthcheck),
}

/**
 * @input =>  we will get docker compose file path to be build/started
 * we will check if it is a valid or correct docker compose.yaml file or not
 * and then we will convert/parse that into docker compose data(version , services ,etc) which we can use to communicate with docker and start the services
 */
pub async fn yaml_parser(
    file_path: impl Into<String>,
    app_state: Arc<cli_memory>,
) -> Result<(), CliErrors> {
    let file_pathh = file_path.into();

    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    // project network and labels(so as to add all containers under 1 folder in docker ui , like docker compose do)
    // we are using current folder der name as label and network name
    let current_dir_name = file_name("", FilePathType::CurrentDir)?;

    // constructing labels for this project
    let mut this_project_labels = create_lables(&current_dir_name).await?;

    // getting service map and service vec in correct dependency order
    let (service_map, service_vec) = validate_file_path(&file_pathh).map_err(|e| e)?;

    // check network and if network.conatiner.len() > 0 , then call restart , else move formward

    // getting network config for existing network
    let networking_config = get_network_config(&current_dir_name);

    //  we have service array and labels(current dir name)(lables are neende dwhile starting image in conatiner)
    let ans = check_and_start_network_containers(
        &docker,
        &current_dir_name,
        &service_vec,
        &service_map,
        &mut this_project_labels,
        &networking_config,
        Arc::clone(&app_state),
    )
    .await?;

    // if ans == false it measn (either no network for this project is present ot conatiners are not present)
    // starting images in conatiner , if they are not present in existing conatiners
    // if network is not present or network has 0 conatiners ,
    // if more that 1  , then we will restart(the existing containers) + build/pull( whose containers are not present)
    // starting images in new conatiners
    if !ans {
        // creating network for this project , so that all the conatiners can be in this network and can communicate with each other
        let this_project_network = create_network(&docker, &current_dir_name).await?;

        // loop over service , and start the images in conatiner 1 by 1
        // if build = . , build current folder , if image name , then pull/build image accordingly
        for ser in service_vec {
            build_or_pull_start_image_in_conatiner(
                &docker,
                ser,
                &service_map,
                &mut this_project_labels,
                &this_project_network,
                Arc::clone(&app_state),
            )
            .await?;
        }
    }

    Ok(())
}

/**
 * @input => input docker compose.yaml file path
 * @reslut => we are validating that file is present and it is a valid docker compose.yaml file
 * and then returing proper formatted map and vec of service is correct dependecy order
 */
pub fn validate_file_path(
    i_file_path: &str,
) -> Result<
    (
        HashMap<String, compose_parser::DockerImageDetails>,
        Vec<String>,
    ),
    CliErrors,
> {
    let file_path = Path::new(i_file_path);
    // string to file path
    let file_name = file_name(i_file_path, FilePathType::FilePath)?;

    // validating file path
    let file_exist = fs::exists(&file_path).map_err(|e| CliErrors::new(e.to_string()))?;
    if !file_exist {
        return Err(CliErrors::wrong_file_path());
    }

    // checking if this input file name exist or not
    let ans = FILE_NAMES.contains(&file_name.as_ref());

    // validating file name
    if !ans {
        return Err(CliErrors::wrong_docker_compose_file_name());
    }

    let file_content = fs::read_to_string(&file_path).map_err(|e| CliErrors::new(e.to_string()))?;

    let compose_content = serde_yaml::from_str::<Compose>(&file_content)
        .map_err(|e| CliErrors::new(e.to_string()))?;

    let services = &compose_content.services;

    // vec and hashmap of sorted serivces
    let ans = construct_docker_image_details_map(&services.0).map_err(|e| e)?;

    Ok((ans.0, ans.1))
}

/**
 * **this function returning labels and newly created network
 * we need this labels to give a tag.label to each conatiner , so that docker can group the conatiners on the basis of this lables
 * we need this network , so docker can put these conatiner in this specified network
 */
pub async fn create_lables(label_name: &str) -> Result<HashMap<String, String>, CliErrors> {
    let mut labels: HashMap<String, String> = HashMap::new();

    // label , under which all the conatiners will come
    labels.insert(
        "com.docker.compose.project".to_string(),
        label_name.to_owned(),
    );

    Ok(labels)
}

/**
 * THIS WILL take current_folder/filepath as input and return the file/folder name in that path
 */
pub fn file_name(i_file_path: &str, file_type: FilePathType) -> Result<String, CliErrors> {
    // if we specify path = current+folder , then we will give current folder nname
    let file_path: PathBuf = match file_type {
        FilePathType::CurrentDir => {
            env::current_dir().map_err(|e| CliErrors::new(format!("{}", { e.to_string() })))?
        }
        // else the name of the file in the path
        FilePathType::FilePath => {
            // string to file path
            PathBuf::from(i_file_path)
        }
    };

    // geting file name from path
    let file_name = file_path
        .file_name()
        .ok_or(CliErrors::file_name_extraction_fail())?;
    let file_name = file_name
        .to_str()
        .ok_or(CliErrors::file_name_extraction_fail())?;

    Ok(file_name.to_string())
}

/**
 * called , when we have to create new network in the docker
 * function to create network for this project
 * and return network config struct
 */
pub async fn create_network(
    docker: &Docker,
    network_name: &str,
) -> Result<NetworkingConfig, CliErrors> {
    // creating new network
    let config = NetworkCreateRequest {
        name: String::from(network_name.to_owned()),
        ..Default::default()
    };
    docker
        .create_network(config)
        .await
        .map_err(|e| CliErrors::new(String::from(format!("{}", { e.to_string() }))))?;

    // constructing networkConfig for the newly created network
    let net_config = get_network_config(network_name);

    Ok(net_config)
}

/**
 * called ,when we just have want networkconfig struct for existing network name
 * this is not creating a new network
 * this will just create and return network config struct
 * just a wrapper around network name
 *
 */
pub fn get_network_config(network_name: &str) -> NetworkingConfig {
    let mut endpoints = HashMap::new();

    endpoints.insert(network_name.to_owned(), EndpointSettings::default());

    let networking_config = NetworkingConfig {
        endpoints_config: Some(endpoints),
    };

    networking_config
}
