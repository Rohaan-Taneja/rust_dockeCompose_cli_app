use bollard::{
    Docker,
    query_parameters::{CreateContainerOptions, InspectContainerOptionsBuilder, ListImagesOptions},
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
    vec,
};
use tokio::{process::Command, task};
use tokio_util::io::ReaderStream;
// use bollard::image::ListImagesOptions;
use docker_compose_types::Compose;
use tar::Builder;

use crate::cli_errors::CliErrors;

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
    Status ,
    Health

}

/**
 * in this we will get file path
 * we will check if it is a valid or correct docker compose.yaml file or not
 * and then we will convert/parse that into docker compose data(version , services ,etc) which we can use to communicate with docker and start the services
 */
pub async fn yaml_parser(file_path: impl Into<String>) -> Result<(), CliErrors> {
    let file_pathh = file_path.into();

    // ### ❓❓ todo
    // if image tag is not present locally , show error image = name not present locally
    // we can do it , see how to get image locally from docker hub and build it and start in conatiner

    // we have docker compose file in compose struct (version service , volumes etc)
    let parsed_yaml_content = validate_file_path(&file_pathh).map_err(|e| e)?;

    // let ans = build_current_folder_image().await.map_err(|e| e)?;
    let ans = start_container("rust_auth_build_with_health_check_curl_added".to_string())
        .await
        .map_err(|e| e)?;

    /**
     * docker provides a client api , though which we can interact get or do what evrr we want to do
     */
    // connection to the
    // let docker = Docker::connect_with_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    // println!("docker connnected {:?}" , &docker);

    //    let version = docker.version().await.unwrap();
    //    println!("this is the version {:?}" , version);
    //        let images = &docker.list_images(Some(ListImagesOptions {
    //     all: true,
    //     ..Default::default()
    // })).await.unwrap();

    // for image in images {
    //     println!("-> {:?}", image);
    // }
    Ok(())
}

/**
 * @input => input docker compose.yaml file path
 * @reslut => we are validating that file is present and it is a valid docker compose.yaml file
 */
pub fn validate_file_path(i_file_path: &str) -> Result<Compose, CliErrors> {
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

    // println!(
    //     "this is the file info {:?} {} {:?}",
    //     file_name, file_content, services
    // );
    Ok(compose_content)
}

/**
 * this function is building current folder to docker image
 * and also consoling the logs
 */
pub async fn build_current_folder_image() -> Result<(), CliErrors> {
    // we can change it according input provided
    let to_be_built_image_tag = String::from("this-project-build");

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
        println!("hellooo we are inside the future iterator");
        match msg {
            Ok(msg) => {
                println!("msg {:?}", msg);
            }
            Err(e) => {
                println!("error in message {e}");
            }
        }
    }

    // let ans = start_container("nginx:latest".to_string()).await.map_err(|e| e)?;

    Ok(())
}

/**
 * deprecated
 * this function is an asyn function it will load current project folder to memory in vetor/buffer (we selectively adding files to the buffer which docker needs)
 * and then converting it into bytes and returning
 * not a good approach because , it is first copying all the files to memory in tar format and we are sharing that
 */
pub async fn convert_to_tar_archive() -> Result<Full<Bytes>, CliErrors> {
    println!("I am in tar build function");

    // copying of files will take time , so added it to a async environment
    // selectively converting the required files into tar format
    let buffer = task::spawn_blocking(move || -> Result<Vec<u8>, CliErrors> {
        let mut buffer = Vec::new();
        {
            let mut builder = Builder::new(&mut buffer);

            // Add src directory
            builder
                .append_dir_all("src", "./src")
                .map_err(|e| CliErrors::new(e.to_string()))?;

            println!("src done");
            // Add files individually
            builder
                .append_path_with_name("./Cargo.toml", "Cargo.toml")
                .map_err(|e| CliErrors::new(e.to_string()))?;

            println!("cargo done");
            builder
                .append_path_with_name("./Cargo.lock", "Cargo.lock")
                .map_err(|e| CliErrors::new(e.to_string()))?;

            builder
                .append_path_with_name("./Dockerfile", "Dockerfile")
                .map_err(|e| CliErrors::new(e.to_string()))?;

            println!("docker done");
            if std::path::Path::new("./.dockerignore").exists() {
                builder
                    .append_path_with_name("./.dockerignore", ".dockerignore")
                    .map_err(|e| CliErrors::new(e.to_string()))?;
            }

            builder
                .finish()
                .map_err(|e| CliErrors::new(e.to_string()))?;
        }
        Ok(buffer)
    })
    .await
    .map_err(|e| CliErrors::new(e.to_string()))?
    .map_err(|e| e)?;

    println!("I am after buider ocntext");

    // full<t> : { data : option(t)}

    let body: Full<Bytes> = Full::new(Bytes::from(buffer));

    return Ok(body);
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
pub async fn start_container(image_tag: String) -> Result<bool, CliErrors> {
    println!(" I am starting the container");
    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    // Define exposed ports (container side)
    let exposed_ports = vec!["8080/tcp".to_string()];

    // Define port bindings (host side)
    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        "8080/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some("8080".to_string()),
        }]),
    );
    println!(" connected with default");

    // Create container and docker port binding with local computer ip and port
    // let container_id = docker
    //     .create_container(
    //         Some(CreateContainerOptions::default()),
    //         ContainerCreateBody {
    //             image: Some(image_tag.to_string()),
    //             exposed_ports: Some(exposed_ports), // conatiner port
    //             host_config: Some(HostConfig {
    //                 port_bindings: Some(port_bindings), // local computer port
    //                 auto_remove: Some(false),
    //                 ..Default::default()
    //             }),
    //             ..Default::default()
    //         },
    //     )
    //     .await
    //     .map_err(|e| {
    //         println!("docker not starting {e}");
    //         CliErrors::new(e.to_string())
    //     })?
    //     .id;

    // println!(" container created , id = {}", &container_id);

    // conatiner starts and function exist , it runs in baclground for now
    // docker
    //     .start_container(
    //         &container_id,
    //         None::<bollard::query_parameters::StartContainerOptions>,
    //     )
    //     .await
    //     .map_err(|e| {
    //         println!("this is the conatiner starting error {e}");

    //         CliErrors::new(e.to_string())
    //     })?;

    println!("container started");

    let res = conatiner_status(&docker, "a59f895a12635b1ab4c51fd1fd3930f665f660074147b70b896bd32f697fc3a4" , ContainerInspectType::Health).await.map_err(|e| e)?;

    Ok(true)
}

/**
 * we will run in loop untill the conatiner starts
 * we will keep seeing the conatiner status untill it is ready
 */
pub async fn wait_until_conatiner_running(
    docker: &Docker,
    container_id: &str,
    
) -> Result<bool, CliErrors> {
    Ok(true)
}

pub async fn conatiner_status(docker: &Docker, container_id: &str ,inspect_type : ContainerInspectType ) -> Result<Option<String>, CliErrors> {
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
            CliErrors::new(e.to_string())})?.state.ok_or_else(|| {
                CliErrors::new("getting none while ispecting the conatiner".to_string())
            })?;

            match inspect_type {
                ContainerInspectType::Status => {

                     let ans_status = container_inspect_response.status.ok_or_else(|| CliErrors::new("getting error in status check".to_string()))?;

                                 println!("this is the container ispect state {:?}" , container_inspect_response.status);
                     return ans_status 

                },
                ContainerInspectType::Health => {

                             let ans_health = container_inspect_response.health;

            println!("this is the container ispect state {:?}" , container_inspect_response.health);

            return ans_health
                }
                
            }

}
