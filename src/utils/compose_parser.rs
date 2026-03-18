use docker_compose_types::{BuildStep, Environment, Healthcheck, Ports, Service};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
  
};

use crate::{cli_errors::CliErrors, utils::service_sort::sort_services, yaml_parser::{FilePathType, file_name}};
use indexmap::IndexMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DockerImageDetails {
    pub build: Option<String>,
    pub image: Option<String>,
    pub container_name: Option<String>,
    pub health_check: Option<Healthcheck>,
    // for now supporting 1 port only (host_port , conatiner_exposed_port) , likning 1 service in container to 1 port outside the container
    //    we may get 1 port(host:con) or no port(for cli type apps , start->run-> end ->conatiner stops)
    pub port: Option<(String, String)>,
    pub environment_vars: Option<HashMap<String, String>>, // env_1 : env_1_value
}

/**
 * @input => we will take compose services indexmap
 * @return => a vec of services in order (for the order of services)
 * and we will also return a hashmap of (service_name -> {service_details/image/to_be_build_image})
 */

pub fn construct_docker_image_details_map(
    services_index_map: &IndexMap<String, Option<Service>>,
) -> Result<(HashMap<String, DockerImageDetails>, Vec<String>), CliErrors> {
    let mut service_map = HashMap::<String, DockerImageDetails>::new();

    // this is the vec conatineing service in correct order (according to dependency graph)
    let mut service_vec = Vec::<String>::new();

    // we will store all the nodes , which we already visited, we will store in it
    let mut visited_ser = HashSet::<String>::new();

    // we will store all the nodes , which we are travelling in 1 go , so that we check cyclic dependecy
    let mut visiting_ser = HashSet::<String>::new();

    for value in services_index_map.keys() {
        // sorting the service is correct dependecy order
        sort_services(
            &mut visiting_ser,
            &mut visited_ser,
            &value,
            &mut service_vec,
            &mut service_map,
            services_index_map,
        )
        .map_err(|e| e)?;
    }

    Ok((service_map, service_vec))
}

/**
 * we will add this service to our serive_map in corrent format
 * we will extract the data from the indexmap and then save it in our DockerImageDetails and then map service : DockerImageDetails
 */
pub fn add_service_to_service_map(
    service_name: &str,
    service_map: &mut HashMap<String, DockerImageDetails>,
    services_details_index_map: &IndexMap<String, Option<Service>>,
) -> Result<bool, CliErrors> {
    let compose_service_data = services_details_index_map
        .get(service_name)
        .ok_or_else(|| CliErrors::cannot_extract_service_details_from_docker_compose())?
        .clone()
        .ok_or_else(|| CliErrors::cannot_extract_service_details_from_docker_compose())?;

    // parsing build folder name from it
    let build_folder = match compose_service_data.build_ {
        Some(BuildStep::Simple(build_path)) => Some(build_path),
        Some(BuildStep::Advanced(_)) => {
            return Err(CliErrors::not_supported_build_type(&service_name));
        }
        None => None,
    };

    let mut image_name = compose_service_data.image;

    let mut container_name = compose_service_data.container_name;

    // we are extracting folder name of this project
    let file_name = file_name("" , FilePathType::CurrentDir)?;

    // projectName_service_name
    if image_name.is_none() {

       
        image_name = Some(format!("{}_{}", file_name, service_name));
        
    }

    // if conatiner name is not specifed , and image and build is bot specified , hen image name is the container name
    if container_name.is_none() {
       
        container_name = Some(format!("{}_{}_1", file_name, service_name));

      
    }

    let healthcheck_data = compose_service_data.healthcheck;

    // extracting ports from the service data
    // Option ((ports tuple)/None(no ports))
    let ports_tuple = match compose_service_data.ports {
        Ports::Short(ports) => {
            if ports.len() > 1 {
                return Err(CliErrors::not_supported_ports_format(&service_name));
            } else if ports.len() == 1 {
                let (h_port, cont_port) = ports
                    .first()
                    .ok_or_else(|| {
                        CliErrors::new(String::from(
                            "please add ports where you want to start service in the container",
                        ))
                    })?
                    .split_once(":")
                    .ok_or_else(|| {
                        CliErrors::new(String::from(
                            "getting error extracting ports of the service",
                        ))
                    })?;

                Some((h_port.to_string(), cont_port.to_string()))
            } else {
                None
            }
        }
        Ports::Long(_) => {
            return Err(CliErrors::not_supported_ports_format(&service_name));
        }
    };

    // hashmap of envs of the image
    let enviroment_vars_hash = match compose_service_data.environment {
        Environment::List(env_list) => {
            let mut env_hash = HashMap::<String, String>::new();

            for env_val in env_list {
                let env_data = env_val.split_once("=").ok_or_else(|| {
                    let message = format!("cannot extract env variables from {service_name} service , please check again");
                    CliErrors::new(String::from(message))})?;

                // addng key value pair of env's to the hash
                env_hash.insert(env_data.0.to_string(), env_data.1.to_string());
            }
            Some(env_hash)
        }
        Environment::KvPair(_) => {
            return Err(CliErrors::new(String::from(
                "this format of environment variables is not supported , please give in list view",
            )));
        }
    };

    let data_struct = DockerImageDetails {
        build: build_folder,
        image: image_name,
        container_name: container_name,
        health_check: healthcheck_data,
        port: ports_tuple,
        environment_vars: enviroment_vars_hash,
    };

    // added service details to the struct
    service_map.insert(service_name.to_string(), data_struct);

    Ok(true)
}
