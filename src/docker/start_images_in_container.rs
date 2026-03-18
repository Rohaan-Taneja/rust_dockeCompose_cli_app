use bollard::models::{EndpointSettings, NetworkingConfig};
use bollard::query_parameters::RestartContainerOptionsBuilder;
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
use git2::Repository;
use http_body_util::StreamBody;
use humantime::parse_duration;
use hyper::body::Frame;
use std::fs;
use std::sync::Arc;
use std::{collections::HashMap, pin::Pin, thread::sleep, time::Duration, vec};
use tokio::process::Command;
use tokio_util::io::ReaderStream;

use crate::cli_memory;
use crate::docker::delete_container::{list_all_filter_conatiners, validate_network};
use crate::logs::service_logs::{
    service_logs, service_logs_messages, service_started, show_pulled_image_specific_logs,
    show_service_error_logs,
};
use crate::utils::check_is_git_repo_url::check_is_git_repo_url;
use crate::utils::compose_parser::DockerImageDetails;
use crate::utils::delete_network::delete_network;
use crate::{cli_errors::CliErrors, yaml_parser::ContainerInspectType};

use futures_util::{Stream, StreamExt};

/**
 * this function is building current folder to docker image
 * and also consoling the logs
 */
pub async fn build_current_folder_image(
    service_name: String,
    container_name : &str,
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
    let body = convert_to_tar_stream(Some(".")).map_err(|e| e)?;

    let mut image_build_stream = docker.build_image(
        build_image_options.build(),
        None,
        Some(http_body_util::Either::Right(body)),
    );

    while let Some(msg) = image_build_stream.next().await {
        match msg {
            Ok(msg) => {
                service_logs(&service_name, msg);
            }
            Err(e) => {
                let error_message = format!(
                    "error while building the current folder image {} => {}",
                    image_tag,
                    e.to_string()
                );
                show_service_error_logs(&service_name, &error_message);
            }
        }
    }

    start_image_in_container(
        &docker,
        container_name,
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
pub async fn pull_image_locally(
    docker: &Docker,
    service_name: String,
    image_name: String,
    app_state: Arc<cli_memory>,
) -> Result<bool, CliErrors> {
    let image_options = Some(CreateImageOptions {
        from_image: Some(image_name.to_string()),
        ..Default::default()
    });

    let mut stream = docker.create_image(image_options, None, None);

    while let Some(pull_result) = stream.next().await {
        match pull_result {
            Ok(res) => {
                show_pulled_image_specific_logs(&image_name, res);
            }
            Err(e) => {
                let error_message = format!(
                    "pulling image => {} , error = {}",
                    image_name,
                    e.to_string()
                );
                show_service_error_logs(&service_name, &error_message);
            }
        }
    }

    service_logs_messages(&service_name, "image pulled successfully");
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
pub fn convert_current_folder_to_tar_stream() -> Result<
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
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CliErrors::new("Failed to capture tar stdout".to_string()))?;

    // it reads the small chunks and emit them out
    let stream = ReaderStream::new(stdout).map(|result| result.map(Frame::data));

    Ok(StreamBody::new(Box::pin(stream)))
}

pub fn convert_to_tar_stream(
    path: Option<&str>,
) -> Result<
    StreamBody<Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, std::io::Error>> + Send>>>,
    CliErrors,
> {
    let mut cmd = Command::new("tar");

    cmd.arg("--exclude=target")
        .arg("--exclude=.git")
        .arg("--no-xattrs")
        .arg("--no-acls")
        .arg("--no-fflags")
        .arg("-cf")
        .arg("-");

    // If path is provided → use it
    // Otherwise → use current directory
    if let Some(p) = path {
        cmd.arg("-C").arg(p).arg(".");
    } else {
        cmd.arg(".");
    }

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| CliErrors::new(e.to_string()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CliErrors::new("Failed to capture tar stdout".to_string()))?;

    let stream = ReaderStream::new(stdout).map(|result| result.map(Frame::data));

    Ok(StreamBody::new(Box::pin(stream)))
}

/**
 * @inputs => this function will recieve an image name/tag
 * @result => we will start that image in a container to specific port
 */
pub async fn start_image_in_container(
    docker: &Docker,
    container_name : &str ,
    service_name: String,
    image_tag: String,
    inspect_type: &ContainerInspectType,
    host_port: Option<String>,
    cont_port: Option<String>,
    network_config: &NetworkingConfig,
    labels: &mut HashMap<String, String>,
    app_state: Arc<cli_memory>,
) -> Result<bool, CliErrors> {
    service_logs_messages(&service_name, "starting image in conatiner");

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

    labels.insert(
        "com.docker.compose.service".to_string(),
        service_name.to_owned(),
    );

    let cont_optins = CreateContainerOptions{
        name : Some(container_name.to_owned()),
        ..Default::default()
    };

    // Create container and docker port binding with local computer ip and port
    let container_id = docker
        .create_container(
            Some(cont_optins),
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
            show_service_error_logs(&service_name, &e.to_string());
            CliErrors::new(e.to_string())
        })?
        .id;

    let service_started_data = format!(" container created , id = {}", &container_id);
    service_logs_messages(&service_name, &service_started_data);

    // conatiner starts and function exist , it runs in baclground for now
    docker
        .start_container(
            &container_id,
            None::<bollard::query_parameters::StartContainerOptions>,
        )
        .await
        .map_err(|e| {
            let err_starting_cont = format!("conatainer starting error {e}");
            show_service_error_logs(&service_name, &err_starting_cont);

            CliErrors::new(e.to_string())
        })?;

    service_logs_messages(&service_name, "container started");

    wait_until_conatiner_running(
        service_name.to_string(),
        &image_tag,
        &docker,
        &container_id,
        inspect_type,
    )
    .await?;

    Ok(true)
}

/**
 * we will run in loop untill the conatiner starts
 * we will keep seeing the conatiner status untill it is ready
 */
pub async fn wait_until_conatiner_running(
    service_name: String,
    image_tag: &str,
    docker: &Docker,
    container_id: &str,
    inspect_type: &ContainerInspectType,
) -> Result<bool, CliErrors> {
    loop {
        // running/healthy
        let res = container_status(
            service_name.to_string(),
            &image_tag,
            &docker,
            &container_id,
            &inspect_type,
        )
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
    service_name: String,
    image_tag: &str,
    docker: &Docker,
    container_id: &str,
    inspect_type: &ContainerInspectType,
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
            show_service_error_logs(&service_name, &e_msg);

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
                service_logs_messages(&service_name, "image is running");
            }

            let cont_status = format!(" image running in container , status = {}", ans_status);
            service_logs_messages(&service_name, &cont_status);

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
            service_logs_messages(&service_name, &cont_health_status);

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

/**
 *this function will check/find all the conatiners lisned to the network
 * and restart all the conatiners present in the service vec,
 * if any service is not present in the container , then we have to build/pull/ start that image in conatiner
 */
pub async fn check_and_start_network_containers(
    docker: &Docker,
    network_id: &str,
    service_vec: &Vec<String>,
    service_map: &HashMap<String, DockerImageDetails>,
    this_project_labels: &mut HashMap<String, String>,
    this_project_network: &NetworkingConfig,
    app_state: Arc<cli_memory>,
) -> Result<bool, CliErrors> {
    // hashmap for service_name -> conatiner_id
    let mut service_to_cont_id_map = HashMap::<String, String>::new();

    println!("hello i am here");

    // if network is valid/exist , the move formward , else
    match validate_network(docker, network_id).await {
        Ok(_) => {
            // if netwrok is there , then we have restart the existing network conatiners
        }
        Err(incoming_err) => {
            let expected_mess = format!(
                "Docker responded with status code 404: network {} not found",
                network_id
            );

            // if netwrok doesnt exist , then no issues , we have to start the images in the conatiner
            // and no restarting
            if incoming_err.message == expected_mess {
                println!("no existing network , we need to start conatiners");

                return Ok(false);
            } else {
                println!("some other network error {:?}", incoming_err);
                return Err(incoming_err);
            }
        }
    }

    // list of all conatiners in this network list
    let network_cont_list = list_all_filter_conatiners(docker, "network", network_id).await?;
    // validate network
    // check if existing network has containers , if yes then restart else go back and start in conatiner

    // if no conatiners in network ,return false and start new images in conatiners
    if network_cont_list.len() == 0 {
        // deleting existing network with no conatiners and returning , so as to create new network and start con in it 
        delete_network(docker, network_id).await?;
        return Ok(false);
    }

    // loop over container list , finding service name running in that cont and creating map
    for cont in &network_cont_list {

        let cont_id = cont
            .id
            .clone()
            .ok_or_else(|| CliErrors::new("unable to extract conatiner id".to_owned()))?;

        // extracting labels map from conatiner summary
        let l = cont.labels.clone().ok_or_else(|| {
            CliErrors::new(format!(
                "not able to extarct lables from conatiner = {} ",
                &cont_id
            ))
        })?;

        if l.get("com.docker.compose.service").is_some() {
            let service = l.get("com.docker.compose.service").unwrap();

            service_to_cont_id_map.insert(service.to_owned(), cont_id.to_owned());
            println!(
                "this is the service name ={:?} for conatiner = {:?} \n \n",
                service, &cont_id
            );
        } else {
            return Err(CliErrors::new(
                "the service name is not present , internal error".to_owned(),
            ));
        }
    }
    println!(
        "this is the map of service -> container id {:?}",
        &service_to_cont_id_map
    );

    // now looping over service vec , in which we have to restart the containers
    // ser is present in (ser -> connt) mapping , then we are restarting that cont
    // if ser is not present , then , it should build/pull/start in the conatiner
    for s in service_vec {
        match service_to_cont_id_map.get(s) {
            Some(c_id) => {
                let ser_details = service_map.get(s).ok_or_else(|| CliErrors::new("serrive details not present in internal struct , please delete all services and re run up command".to_owned()))?;
                let ser_name = ser_details.image.clone().unwrap();
                let health_check_enum = match ser_details.health_check.clone() {
                    Some(health_polling_details) => {
                        ContainerInspectType::Health(health_polling_details)
                    }
                    None => ContainerInspectType::Status,
                };

                restart_container(docker, c_id, s, &ser_name, &health_check_enum).await?;
            }
            None => {
                println!("this service is not present in th existing lable {s}");
                build_or_pull_and_start_image_in_conatiner(
                    docker,
                    s.to_string(),
                    service_map,
                    this_project_labels,
                    this_project_network,
                    Arc::clone(&app_state),
                )
                .await?;
            }
        }
    }
    Ok(true)
}

/**
 *  this function will restart the container
 */
pub async fn restart_container(
    docker: &Docker,
    c_name: &str,
    ser_name: &str,
    image_tag: &str,
    inspect_type: &ContainerInspectType,
) -> Result<(), CliErrors> {
    let c_id = docker
        .restart_container(c_name, None)
        .await
        .map_err(|e| CliErrors::new(e.to_string()))?;

    wait_until_conatiner_running(ser_name.to_owned(), image_tag, docker, c_name, inspect_type)
        .await?;

    service_started(ser_name, "service restarted".to_owned());

    Ok(())
}

/**
 * this will get service name as input and it will check and build/pull/start the image in conatiner
 */
pub async fn build_or_pull_and_start_image_in_conatiner(
    docker: &Docker,
    ser: String,
    service_map: &HashMap<String, DockerImageDetails>,
    this_project_labels: &mut HashMap<String, String>,
    this_project_network: &NetworkingConfig,
    app_state: Arc<cli_memory>,
) -> Result<(), CliErrors> {
    let current_image_details = service_map
        .get(&ser)
        .ok_or_else(|| CliErrors::new(format!("getting some erro whil extracting {ser}")))?;

    let container_name = current_image_details
        .container_name
        .clone()
        .ok_or_else(|| CliErrors::new(format!("no container name found for service =  {ser}")))?;

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

    let image_name = current_image_details.image.clone().ok_or_else(|| CliErrors::new(format!("no image name fount for service {}" , &ser)))?;

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
                build_current_folder_image(
                    ser.to_string(),
                    &container_name ,
                    image_name.to_owned(),
                    &health_check_enum,
                    h_p.to_owned(),
                    c_p.to_owned(),
                    this_project_labels,
                    &this_project_network,
                    Arc::clone(&app_state),
                )
                .await?;
            }
            // if its a url
            // if its a git repo url
            // then call repofunction
            else if check_is_git_repo_url(build_file)? {

                build_remote_git_repo(
                    &build_file,
                    &container_name ,
                    ser.to_string(),
                    image_name.to_owned(),
                    &health_check_enum,
                    h_p.to_owned(),
                    c_p.to_owned(),
                    this_project_labels,
                    &this_project_network,
                    Arc::clone(&app_state),
                )
                .await?;
            } else {
                return Err(CliErrors::new(format!(
                    "currently we are supporting building only current dir and public git repo and images present locally or in docker hub"
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

            // if not present locally, it must be present in at docker hub , we will check there
            if !check_local_image {
                pull_image_locally(
                    &docker,
                    ser.to_string(),
                    image_name.to_owned(),
                    Arc::clone(&app_state),
                )
                .await?;
            }

            start_image_in_container(
                &docker,
                &container_name ,
                ser.to_string(),
                image_name.to_owned(),
                &health_check_enum,
                h_p.to_owned(),
                c_p.to_owned(),
                &this_project_network,
                this_project_labels,
                Arc::clone(&app_state),
            )
            .await?;
        }
    }

    Ok(())
}

/**
 * this functill clone provided git repo and build it and send tar stream ,to crete image of it
 * then that image will be started in the conatiner
 * then we will delete the temp folder creaated(in which the git repo is cloned)
 */
pub async fn build_remote_git_repo(
    git_repo_url: &str,
    container_name : &str,
    service_name: String,
    image_tag: String,
    inspect_type: &ContainerInspectType,
    host_port: Option<String>,
    cont_port: Option<String>,
    this_project_labels: &mut HashMap<String, String>,
    this_project_network: &NetworkingConfig,
    app_state: Arc<cli_memory>,
) -> Result<bool, CliErrors> {
    let temp_path = "./temp/git_pull";

    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;
    let r =
        Repository::clone(git_repo_url, temp_path).map_err(|e| CliErrors::new(e.to_string()))?;

    service_logs_messages(&service_name, "sucessfully pulled the remote git repo");

    let build_image_options = bollard::query_parameters::BuildImageOptionsBuilder::default()
        .dockerfile("Dockerfile")
        .t(&image_tag)
        .pull("true");

    let body = convert_to_tar_stream(Some(temp_path))?;

    let mut image_build_stream = docker.build_image(
        build_image_options.build(),
        None,
        Some(http_body_util::Either::Right(body)),
    );

    service_logs_messages(&service_name, "streaming tar archive to docker for building image");

    while let Some(msg) = image_build_stream.next().await {
        match msg {
            Ok(msg) => {
                service_logs(&service_name, msg);
            }
            Err(e) => {
                 let error_message = format!(
                    "error while building the current folder image {} => {}",
                    image_tag,
                    e.to_string()
                );
                show_service_error_logs(&service_name, &error_message);
            }
        }
    }

    start_image_in_container(
        &docker,
        container_name ,
        service_name,
        image_tag,
        inspect_type,
        host_port.to_owned(),
        cont_port.to_owned(),
        &this_project_network,
        this_project_labels,
        Arc::clone(&app_state),
    )
    .await?;

    // deleteing the temp folder created for temp build
    fs::remove_dir_all("./temp").map_err(|e| CliErrors::new(e.to_string()))?;

    Ok(true)
}
