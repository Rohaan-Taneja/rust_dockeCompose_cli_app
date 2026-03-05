use bollard::{
    Docker,
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, InspectContainerOptionsBuilder,
        ListImagesOptionsBuilder,
    },
    secret::{ContainerCreateBody, HostConfig, PortBinding},
};
use bytes::Bytes;
use http_body_util::StreamBody;
use hyper::body::Frame;
use std::{
    collections::HashMap,
    pin::Pin,
    thread::sleep,
    time::{self, Duration},
    vec,
};
use tokio::process::Command;
use tokio_util::io::ReaderStream;

use crate::{cli_errors::CliErrors, yaml_parser::ContainerInspectType};

use futures_util::{Stream, StreamExt};

/**
 * this function is building current folder to docker image
 * and also consoling the logs
 */
pub async fn build_current_folder_image(
    image_tag: String,
    inspect_type: &ContainerInspectType,
    host_port: Option<String>,
    cont_port: Option<String>,
) -> Result<bool, CliErrors> {
    // we can change it according input provided
    let to_be_built_image_tag = image_tag;

    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    let build_image_options = bollard::query_parameters::BuildImageOptionsBuilder::default()
        .dockerfile("Dockerfile")
        .t(&to_be_built_image_tag)
        .pull("true");

    // geting the tar stream of this folder/project , if service is (build : .)
    // let body = convert_to_tar_archive().await.map_err(|e| e)?;
    let body = convert_current_folder_to_tar_stream().map_err(|e| e)?;

    // println!("i am after tar build call {:?}" , body);

    let mut image_build_stream = docker.build_image(
        build_image_options.build(),
        None,
        Some(http_body_util::Either::Right(body)),
    );

    println!("this is the build stream");
    while let Some(msg) = image_build_stream.next().await {
        match msg {
            Ok(msg) => {
                println!("msg {:?}", msg);
            }
            Err(e) => {
                println!("error in message {e}");
            }
        }
    }

    start_image_in_container(
        &docker,
        to_be_built_image_tag,
        inspect_type,
        host_port.to_owned(),
        cont_port.to_owned(),
    )
    .await?;

    Ok(true)
}

/**
 * @input  => local docker referece and image to pull from docker
 * @result => we will pull the image from the docker hub ans start in the conatiner
 */
pub async fn pull_image_locally(docker: &Docker, image_name: String) -> Result<bool, CliErrors> {
    let image_options = Some(CreateImageOptions {
        from_image: Some(image_name),
        ..Default::default()
    });

    let mut stream = docker.create_image(image_options, None, None);

    while let Some(pull_result) = stream.next().await {
        match pull_result {
            Ok(res) => {
                println!("build steps {res:?}");
            }
            Err(e) => {
                println!("build steps error {e:?}");
            }
        }
    }

    println!("image pulled successfully");
    Ok(true)
}

/**
 * we will check for a image tag , if it is present locally or not
 * @input => image tag
 */
pub async fn check_image_locally(docker: &Docker, image_tag_name: &str) -> Result<bool, CliErrors> {
    let options = ListImagesOptionsBuilder::default().all(true).build();
    let local_images = docker
        .list_images(Some(options))
        .await
        .map_err(|e| CliErrors::new(e.to_string()))?;

    for image in local_images {
        if image.repo_tags.len() > 0
            && image.repo_tags.len() > 0
            && image.repo_tags[0] == image_tag_name
        {
            println!("this is the image {:?}", image.repo_tags[0]);

            return Ok(true);
        }
    }

    Ok(false)
}

/**
 * it will convert current folder into stream of tar format
 * its not a async functon , it will be called and a stream is shared back and it will start streaming data to docker api
 * tar file chunks loads 1 by 1 in memory and send to docker api
 * so no big file converted to tar loaded in meory , no meory spikes , chunk by chunk approach is better that converting and loading all project in temp memory at once
 */
fn convert_current_folder_to_tar_stream() -> Result<
    StreamBody<Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, std::io::Error>> + Send>>>,
    CliErrors,
> {
    // we are using sytem tar command to create current folder to tar and stream it
    let mut child = Command::new("tar")
        .arg("--no-xattrs")
        .arg("--exclude=target") //we are excluding target folder and git folder manually
        .arg("--exclude=.git")
        .arg("--no-acls")
        .arg("--no-fflags")
        .arg("-cf")
        .arg("-")
        .arg(".") //the folder that we want to create tar and stream
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CliErrors::new(e.to_string()))?;

    // some tar it got , it recievs it and it is treamed via readerstream
    let stdout = child.stdout.take().unwrap();

    // it reads the small chunks and emit them out
    let stream = ReaderStream::new(stdout).map(|result| result.map(Frame::data));

    Ok(StreamBody::new(Box::pin(stream)))
}

/**
 * @inputs => this function will recieve an image name/tag
 * @result => we will start that image in a container to specific port
 */
pub async fn start_image_in_container(
    docker: &Docker,
    image_tag: String,
    inspect_type: &ContainerInspectType,
    host_port: Option<String>,
    cont_port: Option<String>,
) -> Result<bool, CliErrors> {
    println!(" I am starting the container");

    // by default , no ports
    // if prots assigned , we will change these configs and give it to the struct below
    let mut exposed_ports: Option<Vec<String>> = None;
    let mut host_config: Option<HostConfig> = None;

    if let (Some(h_p), Some(c_p)) = (host_port, cont_port) {
        let cont_port_key = format!("{c_p}/tcp");
        exposed_ports = Some(vec![cont_port_key.to_owned()]);

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            cont_port_key,
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(h_p),
            }]),
        );

        host_config = Some(HostConfig {
            port_bindings: Some(port_bindings),
            auto_remove: Some(false),
            ..Default::default()
        })
    }

    // Define port bindings (host side)

    println!(" connected with default");

    // let image_name = String::from("our_currnt_p")

    // Create container and docker port binding with local computer ip and port
    let container_id = docker
        .create_container(
            Some(CreateContainerOptions::default()),
            ContainerCreateBody {
                image: Some(image_tag.to_string()),
                exposed_ports: exposed_ports, // conatiner port
                host_config: host_config,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| {
            println!("docker not starting {e}");
            CliErrors::new(e.to_string())
        })?
        .id;

    println!(" container created , id = {}", &container_id);

    // conatiner starts and function exist , it runs in baclground for now
    docker
        .start_container(
            &container_id,
            None::<bollard::query_parameters::StartContainerOptions>,
        )
        .await
        .map_err(|e| {
            println!("this is the conatiner starting error {e}");

            CliErrors::new(e.to_string())
        })?;

    println!("container started");

    // wait_until_conatiner_running(&docker, &container_id, inspect_type)
    //     .await
    //     .map_err(|e| e)?;

    Ok(true)
}

/**
 * we will run in loop untill the conatiner starts
 * we will keep seeing the conatiner status untill it is ready
 */
pub async fn wait_until_conatiner_running(
    docker: &Docker,
    container_id: &str,
    inspect_type: &ContainerInspectType,
) -> Result<bool, CliErrors> {
    loop {
        let res = conatiner_status(&docker, &container_id, &inspect_type)
            .await
            .map_err(|e| e)?;

        println!("this is the res for container status {res}");

        if res == "healthy" {
            break;
        }
        let current_time = time::Instant::now();
        sleep(Duration::new(2, 0));

        let after_time = time::Instant::now();

        println!("current time was {current_time:?} and after time is {after_time:?}");
    }

    Ok(true)
}

/**
 * this function will poll the container and check its status/health(service running or not)
 */
pub async fn conatiner_status(
    docker: &Docker,
    container_id: &str,
    inspect_type: &ContainerInspectType,
) -> Result<String, CliErrors> {
    println!("I am here");
    let container_inspect_response = docker
        .inspect_container(
            container_id,
            Some(
                InspectContainerOptionsBuilder::default()
                    .size(false)
                    .build(),
            ),
        )
        .await
        .map_err(|e| {
            println!("this is the error in starting the conatiner {e}");
            CliErrors::new(e.to_string())
        })?
        .state
        .ok_or_else(|| CliErrors::new("getting none while ispecting the conatiner".to_string()))?;

    println!(
        "I am after conatiner inspect res {:?}",
        &container_inspect_response
    );

    // match inspect type , whetehr we want to check just running conatiner
    // or check if the service is healthy/running or not
    match inspect_type {
        ContainerInspectType::Status => {
            println!("I am in status check");
            let ans_status = container_inspect_response
                .status
                .ok_or_else(|| {
                    CliErrors::new("health check is not defined for this service".to_string())
                })?
                .to_string();

            println!("this is the container ispect state {}", ans_status);

            return Ok(String::from("value"));
        }
        ContainerInspectType::Health => {
            println!("i am in health check");
            let ans_health = container_inspect_response
                .health
                .ok_or_else(|| CliErrors::new("getting error in heath check".to_string()))?
                .status
                .ok_or_else(|| {
                    CliErrors::new(String::from("GETTING ERROR IN HEALTH CHECK STATUS"))
                })?
                .to_string();

            println!("this is the container ispect state {:?}", ans_health);

            return Ok(ans_health);
        }
    }
}
