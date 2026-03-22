use std::collections::{HashMap, HashSet};

use docker_compose_types::{DependsCondition, DependsOnOptions, Service};
use indexmap::IndexMap;

use crate::{
    cli_errors::CliErrors, utils::compose_parser::{DockerImageDetails, add_service_to_service_map},
};

/**
 * this will use topological sort for graph and sort all the dependecies
 * it will travell to graph of services
 */
pub fn sort_services(
    visiting_vec: &mut HashSet<String>,
    visited_vec: &mut HashSet<String>,
    service: &str,
    service_vec: &mut Vec<String>,
    service_map: &mut HashMap<String, DockerImageDetails>,
    services_index_map: &IndexMap<String, Option<Service>>,
    i_file_path : &str
) -> Result<bool, CliErrors> {
    // if. we come to same node which we came acroos just now in this flow , this measn there is a cycle
    // so this cannot be resolved
    // it should be directed acyclic flow
    if visiting_vec.contains(service) {
        return Err(CliErrors::new(String::from(
            "there is a cyclic dependecy in the services , please check and resolve it",
        )));
    }

    // if already visited/travelled node comes again
    // its dependent nodes are already there , so leave it , continue to next
    if visited_vec.contains(service) {
        return Ok(true);
    }

    // since now we are travelling to this node , so we will add this to our visiting vec
    visiting_vec.insert(service.to_string());

    let service_details = services_index_map
        .get(service)
        .ok_or_else(|| {
            CliErrors::new(format!("service = {service} is not present , please check"))
        })?
        .as_ref()
        .ok_or_else(|| CliErrors::new(String::from("value")))?;

    match &service_details.depends_on {
        DependsOnOptions::Simple(depends_on_vec) => {
            if depends_on_vec.len() > 0 {
                for ser in depends_on_vec {
                    // recursive call to dependend node
                    sort_services(
                        visiting_vec,
                        visited_vec,
                        &ser,
                        service_vec,
                        service_map,
                        services_index_map,
                        i_file_path
                    )
                    .map_err(|e| e)?;
                }
            };
        }
        DependsOnOptions::Conditional(dep_map) => {
            if dep_map.len() > 0 {
                for ser in dep_map.keys() {
                    // recursive call to dependend node
                    sort_services(
                        visiting_vec,
                        visited_vec,
                        &ser,
                        service_vec,
                        service_map,
                        services_index_map,
                        i_file_path
                    )
                    .map_err(|e| e)?;
                }
            };
        }
    }

    // adding service to our data structure for our usage
    service_vec.push(service.to_string());
    add_service_to_service_map(service, service_map, services_index_map , i_file_path).map_err(|e| e)?;

    // since we have already gone though it , so adding , so that if comes again , we can directly return okay and not traverse again
    visited_vec.insert(service.to_owned());

    // since we have gone through it , so this is already visited , so removing it from visiting vec
    visiting_vec.remove(service);

    Ok(true)
}
