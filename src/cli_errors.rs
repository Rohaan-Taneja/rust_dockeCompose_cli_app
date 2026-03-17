// we will create a struct to dispaly cli errors .

use bollard::errors::Error;

#[derive(Debug, Clone)]
pub struct CliErrors {
    pub message: String,
}

impl CliErrors {
    pub fn new(message: String) -> CliErrors {
        return CliErrors { message };
    }

    pub fn wrong_cli_name() -> CliErrors {
        return CliErrors {
            message: "wrong cli command , try with DockYard ...".to_string(),
        };
    }

    pub fn wrong_cli_command() -> CliErrors {
        return CliErrors {
            message:
                "this cli command is not supported by dockyard , try Up , Down , Status , Logs"
                    .to_string(),
        };
    }

    pub fn file_name_extraction_fail() -> CliErrors {
        return CliErrors {
            message: "we are getting while extracting file".to_string(),
        };
    }

    pub fn wrong_docker_compose_file_name() -> CliErrors {
        return CliErrors {
            message: "the input docker compose file name is not correct , please check file name"
                .to_string(),
        };
    }

    pub fn wrong_file_path() -> CliErrors {
        return CliErrors {
            message: "wrong file path".to_string(),
        };
    }

    pub fn cannot_extract_service_details_from_docker_compose() -> CliErrors {
        return CliErrors {
            message: String::from(
                "cannot extract service details from the docker compose file , please check",
            ),
        };
    }

    pub fn not_supported_build_type(service_name: &str) -> CliErrors {
        return CliErrors {
            message: String::from(format!(
                "error in service =>{service_name}, we dont support this build format , only simple build is supported like this folder or folder path to build"
            )),
        };
    }

    pub fn not_supported_ports_format(service_name: &str) -> CliErrors {
        return CliErrors::new(format!(
            "Error in service `{service_name}`: Invalid ports configuration. \
     Only one port mapping in the format `host_port:container_port` is supported."
        ));
    }
}
