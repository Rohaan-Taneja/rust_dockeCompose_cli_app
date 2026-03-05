use bollard::{
    Docker,
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, InspectContainerOptionsBuilder,
        ListImagesOptions, ListImagesOptionsBuilder,
    },
    secret::{Config, ContainerCreateBody, HostConfig, PortBinding},
};
use bytes::Bytes;
use http_body_util::{Full, StreamBody};
use hyper::body::Frame;
use std::{
    collections::HashMap,
    fs::{self},
    io::{self, Error},
    path::Path,
    pin::Pin,
    process::Stdio,
    thread::sleep,
    time::{self, Duration},
    vec,
};
use tokio::{process::Command, task};
use tokio_util::io::ReaderStream;
// use bollard::image::ListImagesOptions;
use docker_compose_types::Compose;
use tar::Builder;

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

use futures_util::{
    Stream, StreamExt, TryFutureExt,
    stream::{self},
};

pub const FILE_NAMES: [&str; 6] = [
    "compose.yaml",
    "compose.yml",
    "docker-compose.yaml",
    "docker-compose.yml",
    "docker-compose.yml",
    "docker-compose.override.yml",
];

/**
 * this will tell what do we have to inspect
 * status =>just to check if the conatiner is running or not
 * health => or to chec is conatiner service has stated or not
 */
pub enum ContainerInspectType {
    Status,
    Health,
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

    // we will validate dockercompose file pat and
    // it will return service details in map and service vec , in correct dependecy format
    let (service_map, service_vec) = validate_file_path(&file_pathh).map_err(|e| e)?;

    let inspect_type = ContainerInspectType::Status;

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

        // starting container , whether it is to be build or start image(local or docker image)
        match &current_image_details.build {
            // build can be  (. / folder path)
            Some(build_file) => {
                if build_file == "." {
                    println!("this is the conatiner name {conatiner_name}");
                    build_current_folder_image(
                        conatiner_name.to_owned(),
                        &inspect_type,
                        h_p.to_owned(),
                        c_p.to_owned(),
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
                    &inspect_type,
                    h_p.to_owned(),
                    c_p.to_owned(),
                )
                .await?;
            }
        }
    }

    Ok(())
}

/**
 * @input => input docker compose.yaml file path
 * @reslut => we are validating that file is present and it is a valid docker compose.yaml file
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
    // string to file path
    let file_path = Path::new(i_file_path);

    // geting file name from path
    let file_name = file_path
        .file_name()
        .ok_or(CliErrors::file_name_extraction_fail())?;
    let file_name = file_name
        .to_str()
        .ok_or(CliErrors::file_name_extraction_fail())?;

    // validating file path
    let file_exist = fs::exists(&file_path).map_err(|e| CliErrors::new(e.to_string()))?;
    if !file_exist {
        return Err(CliErrors::wrong_file_path());
    }

    // checking if this input file name exist or not
    let ans = FILE_NAMES.contains(&file_name);

    // validating file name
    if !ans {
        return Err(CliErrors::wrong_docker_compose_file_name());
    }

    let file_content = fs::read_to_string(&file_path).map_err(|e| CliErrors::new(e.to_string()))?;

    let compose_content = serde_yaml::from_str::<Compose>(&file_content)
        .map_err(|e| CliErrors::new(e.to_string()))?;

    let services = &compose_content.services;

    // println!("this is the file info {} {:?}", file_content, &services);

    // vec and hashmap of sorted serivces
    let ans = construct_docker_image_details_map(&services.0).map_err(|e| e)?;

    Ok((ans.0, ans.1))
}
