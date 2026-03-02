use docker_compose_types::{BuildStep, DependsOnOptions, Environment, Healthcheck, Ports, Service};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cli_errors::CliErrors;
use indexmap::IndexMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DockerImageDetails {
    pub build: Option<String>,
    pub image: Option<String>,
    pub container_name: Option<String>,
    pub health_check: Option<Healthcheck>,
    // for now supporting 1 port only (host_port , conatiner_exposed_port) , likning 1 service in container to 1 port outside the container
    pub port: (String, String),
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

    let mut service_vec = Vec::<String>::new();

    for (value, service) in services_index_map {
        // service details option
        match service {
            Some(service_details) => {
                // if service from index map is present in our service_vec
                // we will then add its depeds on to service_vec (before this service) and and to service_map
                if service_map.contains_key(value) {
                    // it will check/add depeds on services before this current service in vec + map
                    check_and_add_depends_on_before_current_service(
                        &value,
                        &service_details.depends_on,
                        &mut service_vec,
                        &mut service_map,
                        &services_index_map,
                    )
                    .map_err(|e| e)?;
                }
                // if service from index map is not present in our hashmap,
                // then we will add its depends_on service
                // then this current service
                else {
                    // add depends on services to the map and vec
                    add_depends_on_to_map_and_vec(
                        &service_details.depends_on,
                        &mut service_vec,
                        &mut service_map,
                        services_index_map,
                    )
                    .map_err(|e| e)?;
                
                    // then add this service to map  and vec
                    service_vec.push(value.to_string());

                    add_service_to_service_map(value, &mut service_map, &services_index_map)
                        .map_err(|e| e)?;
                }
            }
            None => {
                println!("the service details not found , value => {value}");
                return Err(CliErrors::new(String::from("service details not found")));
            }
        }
    }

    Ok((service_map, service_vec))
}

/**
 * **(current_service already in service_vec )adding/checking depends on services before the current_service in service vec
 *
 * here we will loop over service , check if depends_on service is present or not
 * if it is present , then it should be present before the current service in vec , if not then pop it and add it before
 * if if it is not present , then add it or alter add it int eh vec and hashmap (add here via a index map from input ,make a different function add the details in hashmap)
 */
pub fn check_and_add_depends_on_before_current_service(
    current_service: &str,
    depends_on: &DependsOnOptions,
    service_vec: &mut Vec<String>,
    service_map: &mut HashMap<String, DockerImageDetails>,
    services_index_map: &IndexMap<String, Option<Service>>,
) -> Result<bool, CliErrors> {
    match depends_on {
        DependsOnOptions::Simple(depeds_on_vec) => {
            if depeds_on_vec.len() > 0 {
                for value in depeds_on_vec {
                    // if this depends on service is already present in our service vec +map
                    if service_map.contains_key(value) {
                        // this depeds on service also present in service_ves , so we will check/reorder it and add it before current_service
                        check__and_update__before_or_after(current_service, value, service_vec);
                    }
                    // this depends on service not present in service_ve/map
                    else {
                        // this will add this depends_on_service to service_vec and map
                        add_depeds_on_service_before_current_service(
                            current_service,
                            value,
                            service_vec,
                            service_map,
                            services_index_map,
                        )
                        .map_err(|e| e)?;
                    }
                }

                Ok(true)
            } else {
                Ok(true)
            }
        }
        DependsOnOptions::Conditional(depends_on_map) => {
            if depends_on_map.len() > 0 {
                for key in depends_on_map.keys() {
                    // if this depends on service is already present in our service vec +map
                    if service_map.contains_key(key) {
                        // this depeds on service also present in service_ves , so we will check/reorder it and add it before current_service
                        check__and_update__before_or_after(current_service, key, service_vec);
                    }
                    // this depends on service not present in service_ve/map
                    else {
                        // this will add this depends_on_service to service_vec and map
                        add_depeds_on_service_before_current_service(
                            current_service,
                            key,
                            service_vec,
                            service_map,
                            services_index_map,
                        )
                        .map_err(|e| e)?;
                    }
                }
                Ok(true)
            } else {
                Ok(true)
            }
        }
    }
}

// [ s1 , s5 , s2 , s4 , s3]
// [ s1 , s2 , s4 , s5 , s3]
// here we know that current and to check both are present
/**
 *  **here we will reorder the to_check_service (before current_service) if needed
 * both service are present
 * if to_check_service is present after current_Service then we will remove it and add it before. current_Service
 * so now according to depeds on array to_check_service will run first ,then current_service
 */
pub fn check__and_update__before_or_after(
    current_service: &str,
    to_check_service: &str,
    service_vec: &mut Vec<String>,
) {
    let mut check = 0;
    let mut before = false;

    let mut current_index = 0;

    // took clone of vector becasue if we loop direclty over &vec , then in else statement we coundt tak mut referece of service_vec (borrow check) infine immutable ref but
    // only 1 mut red
    for (index, v) in service_vec.clone().iter().enumerate() {
        if v == to_check_service {
            //  we found the depends on service and we havent found currenr_service till now , it measn to_check service is beofre current_Service , so it is good/ok
            if check == 0 {
                before = true;
                break;
            }
            // we found the depneds on service and check == 1 , means , we have already found the current_service before to_check_service , so we need to pop this and add it after this current service
            else {
                // removed the to_check_service from that position
                service_vec.remove(index);

                // added the to_check_service before the current_service
                service_vec.insert(current_index, to_check_service.to_string());

                break;
            }
        } else if v == current_service {
            check = 1;
            current_index = index;
        }
    }

    // before telling found beofre or after
}

/**
 * **current_service present , we will add dspecific deepends_on service before it
 * add depeds on services beofre current_Service in service_vec and also in the service_map
 * and add the depends_on_service before it and also add that service to service map also
 *
 */
pub fn add_depeds_on_service_before_current_service(
    current_service: &str,
    depends_on_service: &str,
    service_vec: &mut Vec<String>,
    service_map: &mut HashMap<String, DockerImageDetails>,
    services_index_map: &IndexMap<String, Option<Service>>,
) -> Result<bool, CliErrors> {
    // loop through service_vec => find current_service => add depeds_on_service before it => and also to the service_map
    for (index, value) in service_vec.clone().iter().enumerate() {
        // we are adding depends on services before the current service in service_vec and also addidng it to the service_map
        if value == current_service {
            service_vec.insert(index, depends_on_service.to_string());

            // add this depends on service to service_map also
            add_service_to_service_map(&depends_on_service, service_map, &services_index_map)
                .map_err(|e| e)?;

            break;
        }
    }

    Ok(true)
}

/**
 * ** this service will add depends on services to our service vec and map
 * this will be called when current service is not in our service vec/map
 * so we will push/add dpends on services to our vec/map which are not already present
 */
pub fn add_depends_on_to_map_and_vec(
    depends_on: &DependsOnOptions,
    service_vec: &mut Vec<String>,
    service_map: &mut HashMap<String, DockerImageDetails>,
    services_index_map: &IndexMap<String, Option<Service>>,
) -> Result<bool, CliErrors> {
    match depends_on {
        DependsOnOptions::Simple(list) => {
            if list.len() > 0 {
                for ser in list {
                    // if list iten is not already present in the service_vec/map , we will add it , else continue to next
                    if !service_map.contains_key(ser) {
                        // added depends on to service vec
                        service_vec.push(ser.to_string());

                        // adding this depeds on service to service_map also
                        add_service_to_service_map(ser, service_map, services_index_map)
                            .map_err(|e| e)?;
                    }
                }
            }
        }

        DependsOnOptions::Conditional(map_data) => {
            if map_data.len() > 0 {
                for ser in map_data.keys() {
                    // if list iten is not already present in the service_vec/map , we will add it , else continue to next
                    if !service_map.contains_key(ser) {
                        // added depends on to service vec
                        service_vec.push(ser.to_string());

                        // adding this depeds on service to service_map also
                        add_service_to_service_map(ser, service_map, services_index_map)
                            .map_err(|e| e)?;
                    }
                }
            }
        }
    }

    Ok(true)
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
    let compose_service_data = services_details_index_map.get(service_name)
        .ok_or_else(|| CliErrors::cannot_extract_service_details_from_docker_compose())?
        .clone()
        .ok_or_else(|| CliErrors::cannot_extract_service_details_from_docker_compose())?;

    // parsing build folder name from it
    let build_folder = match compose_service_data.build_ {
        Some(BuildStep::Simple(build_path)) => Some(build_path),
        Some(BuildStep::Advanced(_)) => {
            return Err(CliErrors::not_supported_build_type(&service_name));
        }
        None => {
            println!("build file name of this service {service_name} is null");
            None
        }
    };

    let image_name = compose_service_data.image;

    let mut container_name = compose_service_data.container_name;

    // if conatiner name is not specifed , and image and build is bot specified , hen image name is the container name
    if container_name.is_none() {
        if image_name.is_some() && build_folder.is_some() {
            container_name = image_name.clone();
        }
    }

    let healthcheck_data = compose_service_data.healthcheck;

    // extracting ports from the service data
    let ports_tuple = match compose_service_data.ports {
        Ports::Short(ports) => {
            if ports.len() > 1 {
                return Err(CliErrors::not_supported_ports_format(&service_name));
            }
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

            (h_port.to_string(), cont_port.to_string())
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
