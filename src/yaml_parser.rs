use bollard::{
    Docker,
    secret::{EndpointSettings, NetworkCreateRequest, NetworkingConfig},
};

use std::{
    collections::HashMap,
    env,
    fs::{self},
    path::{Path, PathBuf},
};

use docker_compose_types::{Compose, Healthcheck};

use crate::{
    cli_errors::CliErrors,
    docker::{
        compose_parser::{self, construct_docker_image_details_map},
        start_images_in_container::{
            build_current_folder_image, check_image_locally, pull_image_locally,
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
pub async fn yaml_parser(file_path: impl Into<String>) -> Result<(), CliErrors> {
    let file_pathh = file_path.into();

    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    // project network and labels(so as to add all containers under 1 folder in docker ui , like docker compose do)
    let current_dir_name = file_name("", FilePathType::CurrentDir)?;
    println!("this is the current dir name {}", &current_dir_name);
    let (mut this_project_labels, this_project_network) =
        create_network(&docker, current_dir_name).await?;

    let this_compose_label = this_project_labels
        .get("com.docker.compose.project")
        .ok_or_else(|| CliErrors::new(String::from("getting erro while fetching the label ")))?;

    let (service_map, service_vec) =
        validate_file_path(&file_pathh, this_compose_label.to_owned()).map_err(|e| e)?;

    // loop over service , and start the images in conatiner 1 by 1
    // if build = . , build current folder , if image name , then pull/build image accordingly
    for ser in service_vec {
        let current_image_details = service_map
            .get(&ser)
            .ok_or_else(|| CliErrors::new(format!("getting some erro whil extracting {ser}")))?;

        let conatiner_name = current_image_details
            .container_name
            .clone()
            .ok_or_else(|| {
                CliErrors::new(format!("no container name found for service =  {ser}"))
            })?;

        // ports from the service
        let mut h_p = None;
        let mut c_p = None;
        match current_image_details.port.clone() {
            Some(prts) => {
                h_p = Some(prts.0);
                c_p = Some(prts.1);
            }
            // ports are none
            None => {}
        }

        // constructing heath check details
        let health_check_enum = match current_image_details.health_check.clone() {
            Some(health_polling_details) => ContainerInspectType::Health(health_polling_details),
            None => ContainerInspectType::Status,
        };

        // starting container , whether it is to be build or start image(local or docker image)
        match &current_image_details.build {
            // build can be  (. / folder path)
            Some(build_file) => {
                if build_file == "." {
                    println!("this is the conatiner name {conatiner_name}");
                    build_current_folder_image(
                        conatiner_name.to_owned(),
                        &health_check_enum,
                        h_p.to_owned(),
                        c_p.to_owned(),
                        &mut this_project_labels,
                        &this_project_network,
                    )
                    .await?;
                } else {
                    return Err(CliErrors::new(format!(
                        "currently we are supporting building only current or images present locally or in docker hub"
                    )));
                }
            }
            // if build is none , then image will be there either local image or we will get it from docker
            None => {
                // if image is present locally , we will run it directly in the conatiner , else check in docker hub , if present there , pull it and then we will call start image in conatiner
                let image_name = current_image_details.image.clone().ok_or_else(|| {
                    CliErrors::new(format!("no image name is provided for service => {ser}"))
                })?;

                let check_local_image = check_image_locally(&docker, &image_name).await?;
                println!("this i the locall image ans {check_local_image}");

                // if not present locally, it must be present in at docker hub , we will check there
                if !check_local_image {
                    pull_image_locally(&docker, image_name.to_owned()).await?;
                }

                start_image_in_container(
                    &docker,
                    image_name.to_owned(),
                    &health_check_enum,
                    h_p.to_owned(),
                    c_p.to_owned(),
                    &this_project_network,
                    &mut this_project_labels,
                )
                .await?;
            }
        }

        println!("this is the current runned service {}", ser);
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
    label: String,
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

    println!("this is the compose content {:?} ", compose_content);

    let services = &compose_content.services;

    // println!("this is the file info {} {:?}", file_content, &services);

    // vec and hashmap of sorted serivces
    let ans = construct_docker_image_details_map(&services.0).map_err(|e| e)?;

    println!("these are the services {ans:?}");

    Ok((ans.0, ans.1))
}

/**
 * **this function returning labels and newly created network
 * we need this labels to give a tag.label to each conatiner , so that docker can group the conatiners on the basis of this lables
 * we need this network , so docker can put these conatiner in this specified network
 */
pub async fn create_network(
    docker: &Docker,
    network_name: String,
) -> Result<(HashMap<String, String>, NetworkingConfig), CliErrors> {
    let config = NetworkCreateRequest {
        name: String::from(network_name.to_owned()),
        ..Default::default()
    };
    docker
        .create_network(config)
        .await
        .map_err(|e| CliErrors::new(String::from(format!("{}", { e.to_string() }))))?;

    let mut labels: HashMap<String, String> = HashMap::new();

    // label , under which all the conatiners will come
    labels.insert(
        "com.docker.compose.project".to_string(),
        network_name.to_owned(),
    );

    let mut endpoints = HashMap::new();

    endpoints.insert(network_name.to_owned(), EndpointSettings::default());

    
    let networking_config = NetworkingConfig {
        endpoints_config: Some(endpoints),
    };

    Ok((labels, networking_config))
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
