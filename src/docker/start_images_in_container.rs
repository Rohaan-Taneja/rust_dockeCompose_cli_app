use bollard::models::{EndpointSettings, NetworkingConfig};
use bollard::secret::{HealthConfig, NetworkCreateRequest};
use bollard::{
    Docker,
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, InspectContainerOptionsBuilder,
        ListImagesOptionsBuilder,
    },
    secret::{ContainerCreateBody, HostConfig, PortBinding},
};

use bytes::Bytes;
use docker_compose_types::{Healthcheck, HealthcheckTest};
use http_body_util::StreamBody;
use humantime::parse_duration;
use hyper::body::Frame;
use std::sync::Arc;
use std::{
    collections::HashMap,
    pin::Pin,
    thread::sleep,
    time::{self, Duration},
    vec,
};
use tokio::process::Command;
use tokio_util::io::ReaderStream;

use crate::cli_memory;
use crate::logs::service_logs::{
    service_logs, service_logs_messages, service_started, show_pulled_image_specific_logs,
    show_service_error_logs,
};
use crate::{cli_errors::CliErrors, yaml_parser::ContainerInspectType};

use futures_util::{Stream, StreamExt};

/**
 * this function is building current folder to docker image
 * and also consoling the logs
 */
pub async fn build_current_folder_image(
    service_name : String,
    image_tag: String,
    inspect_type: &ContainerInspectType,
    host_port: Option<String>,
    cont_port: Option<String>,
    this_project_labels: &mut HashMap<String, String>,
    this_project_network: &NetworkingConfig,
    app_state: Arc<cli_memory>,
) -> Result<bool, CliErrors> {
    // we can change it according input provided
    let to_be_built_image_tag = image_tag.to_string();

    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    let build_image_options = bollard::query_parameters::BuildImageOptionsBuilder::default()
        .dockerfile("Dockerfile")
        .t(&to_be_built_image_tag)
        .pull("true");

    // geting the tar stream of this folder/project , if service is (build : .)
    // let body = convert_to_tar_archive().await.map_err(|e| e)?;
    let body = convert_current_folder_to_tar_stream().map_err(|e| e)?;

    let mut image_build_stream = docker.build_image(
        build_image_options.build(),
        None,
        Some(http_body_util::Either::Right(body)),
    );

    while let Some(msg) = image_build_stream.next().await {
        match msg {
            Ok(msg) => {
                service_logs(&service_name, msg, Arc::clone(&app_state));
            }
            Err(e) => {
                let error_message = format!(
                    "error while building the current folder image {} => {}",
                    image_tag,
                    e.to_string()
                );
                show_service_error_logs(&service_name, &error_message, Arc::clone(&app_state));
            }
        }
    }

    start_image_in_container(
        &docker,
        service_name,
        to_be_built_image_tag,
        inspect_type,
        host_port.to_owned(),
        cont_port.to_owned(),
        &this_project_network,
        this_project_labels,
        Arc::clone(&app_state),
    )
    .await?;

    Ok(true)
}

/**
 * @input  => local docker referece and image to pull from docker
 * @result => we will pull the image from the docker hub ans start in the conatiner
 */
pub async fn pull_image_locally(docker: &Docker, service_name : String ,  image_name: String , app_state: Arc<cli_memory>) -> Result<bool, CliErrors> {
    let image_options = Some(CreateImageOptions {
        from_image: Some(image_name.to_string()),
        ..Default::default()
    });

    let mut stream = docker.create_image(image_options, None, None);

    while let Some(pull_result) = stream.next().await {
        match pull_result {
            Ok(res) => {
                show_pulled_image_specific_logs(&image_name, res , Arc::clone(&app_state));
            }
            Err(e) => {
                let error_message = format!(
                    "pulling image => {} , error = {}",
                    image_name,
                    e.to_string()
                );
                show_service_error_logs(&service_name, &error_message , Arc::clone(&app_state));
            }
        }
    }

    service_logs_messages(&service_name, "image pulled successfully" , Arc::clone(&app_state));
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
    service_name : String,
    image_tag: String,
    inspect_type: &ContainerInspectType,
    host_port: Option<String>,
    cont_port: Option<String>,
    network_config: &NetworkingConfig,
    labels: &mut HashMap<String, String>,
    app_state: Arc<cli_memory>,
) -> Result<bool, CliErrors> {
    service_logs_messages(&service_name, "starting image in conatiner", Arc::clone(&app_state));

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

    // if health check(healthy) is given the we will give data else none is given
    let health_config: Option<HealthConfig> = match inspect_type {
        ContainerInspectType::Status => None,
        ContainerInspectType::Health(health_inspect_data) => {
            // Some(health_inspect_data.clone())
            // put data in correct format
            let health_config_struct = get_health_config(health_inspect_data.clone())?;

            Some(health_config_struct)
        }
    };

    labels.insert("com.docker.compose.service".to_string(), image_tag.clone());

    // Create container and docker port binding with local computer ip and port
    let container_id = docker
        .create_container(
            Some(CreateContainerOptions::default()),
            ContainerCreateBody {
                image: Some(image_tag.to_string()),
                exposed_ports: exposed_ports, // conatiner port
                host_config: host_config,
                labels: Some(labels.clone()),
                networking_config: Some(network_config.clone()),
                healthcheck: health_config,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| {
            show_service_error_logs(&service_name, &e.to_string() , Arc::clone(&app_state));
            CliErrors::new(e.to_string())
        })?
        .id;

    let service_started_data = format!(" container created , id = {}", &container_id);
    service_logs_messages(&service_name, &service_started_data , Arc::clone(&app_state));

    // conatiner starts and function exist , it runs in baclground for now
    docker
        .start_container(
            &container_id,
            None::<bollard::query_parameters::StartContainerOptions>,
        )
        .await
        .map_err(|e| {
            let err_starting_cont = format!("conatainer starting error {e}");
            show_service_error_logs(&service_name, &err_starting_cont , Arc::clone(&app_state));

            CliErrors::new(e.to_string())
        })?;

    service_logs_messages(&service_name, "container started" , Arc::clone(&app_state));

    wait_until_conatiner_running(service_name.to_string() , &image_tag, &docker, &container_id, inspect_type ,Arc::clone(&app_state) ).await?;

    Ok(true)
}

/**
 * we will run in loop untill the conatiner starts
 * we will keep seeing the conatiner status untill it is ready
 */
pub async fn wait_until_conatiner_running(
    service_name : String,
    image_tag: &str,
    docker: &Docker,
    container_id: &str,
    inspect_type: &ContainerInspectType,
    app_state: Arc<cli_memory>
) -> Result<bool, CliErrors> {
    loop {
        // running/healthy
        let res = container_status( service_name.to_string() , &image_tag, &docker, &container_id, &inspect_type , Arc::clone(&app_state))
            .await
            .map_err(|e| e)?;

        // for status , if status is running break and go
        // health , if status is healthy , break and go
        match inspect_type {
            ContainerInspectType::Status => {
                // if polling for status , status should be runnning , if not , wait

                if res == "running".to_owned() {
                    break;
                }
            }
            // if polling for healthy service , health should be healthy , if not , wait
            ContainerInspectType::Health(_) => {
                if res == "healthy".to_owned() {
                    break;
                }

                
                sleep(Duration::from_secs(3));
            }
        }

    }

    Ok(true)
}

/**
 * this function will poll the container and check its status/health(service running or not)
 */
pub async fn container_status(
    service_name : String,
    image_tag: &str,
    docker: &Docker,
    container_id: &str,
    inspect_type: &ContainerInspectType,
    app_state: Arc<cli_memory>
) -> Result<String, CliErrors> {
    // polling and getting conatiner status
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
            let e_msg = format!("error in starting the conatiner {e}");
            show_service_error_logs(&service_name, &e_msg , Arc::clone(&app_state));

            CliErrors::new(e.to_string())
        })?
        .state
        .ok_or_else(|| CliErrors::new("getting none while inspecting the conatiner".to_string()))?;

    // match inspect type , whetehr we want to check just running conatiner
    // or check if the service is healthy/running or not
    match inspect_type {
        ContainerInspectType::Status => {
            let ans_status = container_inspect_response
                .status
                .ok_or_else(|| {
                    CliErrors::new("health check is not defined for this service".to_string())
                })?
                .to_string();

            if ans_status == "running".to_string() {
                service_logs_messages(&service_name, "image is running" , Arc::clone(&app_state));
            }

            let cont_status = format!(" image running in container , status = {}", ans_status);
            service_logs_messages(&service_name, &cont_status , Arc::clone(&app_state));

            Ok(ans_status)
        }
        ContainerInspectType::Health(polling_details) => {
            let ans_health = container_inspect_response
                .health
                .ok_or_else(|| CliErrors::new("getting error in heath check".to_string()))?
                .status
                .ok_or_else(|| {
                    CliErrors::new(String::from("GETTING ERROR IN HEALTH CHECK STATUS"))
                })?
                .to_string();

            let cont_health_status = format!(
                "image running in conatiner, health check = {:?}",
                ans_health
            );
            service_logs_messages(&service_name, &cont_health_status , Arc::clone(&app_state));

            return Ok(ans_health);
        }
    }
}

/**
 * this function will take healthcheck struct ans input and return healthconfig struct as output
 */
pub fn get_health_config(health_check: Healthcheck) -> Result<HealthConfig, CliErrors> {
    // test command in vec format
    let test = match health_check.test {
        Some(HealthcheckTest::Single(s)) => Some(vec![s]),
        Some(HealthcheckTest::Multiple(vec_tests)) => Some(vec_tests),
        None => None,
    };

    let interval = match health_check.interval {
        Some(i) => {
            Some(duration_to_nanos(&i)?)
            // let timeout = duration_to_nanos("5s")?;
            // let start_period = duration_to_nanos("10s")?;
        }
        None => None,
    };

    let timeout = match health_check.timeout {
        Some(ref t) => Some(duration_to_nanos(&t)?),
        None => None,
    };

    let start_interval = match health_check.start_interval {
        Some(si) => Some(duration_to_nanos(&si)?),
        None => None,
    };

    let start_period = match health_check.start_period {
        Some(ref sp) => Some(duration_to_nanos(&sp)?),
        None => None,
    };

    let retries = Some(health_check.retries);

    Ok(HealthConfig {
        test: test,
        interval: interval,
        timeout: timeout,
        retries: retries,
        start_period: start_period,
        start_interval: start_interval,
    })
}

// function to convert time to nanno seconds
pub fn duration_to_nanos(input: &str) -> Result<i64, CliErrors> {
    let duration: Duration =
        parse_duration(input).map_err(|e| CliErrors::new(format!("{}", e.to_string())))?;
    Ok(duration.as_nanos() as i64)
}
