use bollard::{
    Docker,
    query_parameters::{InspectContainerOptions, InspectContainerOptionsBuilder},
    secret::{Health, HealthStatusEnum, RestartPolicyNameEnum},
};

use crate::cli_errors::CliErrors;

pub async fn docker_conatiner_status(container_id: String) -> Result<bool, CliErrors> {
    let docker =
        Docker::connect_with_local_defaults().map_err(|e| CliErrors::new(e.to_string()))?;

    let container = docker
        .inspect_container(
            &container_id,
            Some(
                InspectContainerOptionsBuilder::default()
                    .size(false)
                    .build(),
            ),
        )
        .await
        .map_err(|e| CliErrors::new(e.to_string()))?;

    println!("this container {:?}", container);

    let state = container
        .state
        .ok_or_else(|| CliErrors::new("Missing container state".to_string()))?;

    let config = container.config;
    let host_config = container.host_config;
    let network_settings = container.network_settings;

    println!("\nContainer Status");
    println!("================\n");

    println!("ID            : {}", container.id.unwrap_or_default());
    println!(
        "Name          : {}",
        container.name.unwrap_or_default().trim_start_matches('/')
    );
    println!(
        "Image         : {}",
        config
            .as_ref()
            .and_then(|c| c.image.clone())
            .unwrap_or_default()
    );
    println!("Created       : {}", container.created.unwrap_or_default());

    let platform = container
        .image_manifest_descriptor
        .as_ref()
        .and_then(|d| d.platform.as_ref())
        .map(|p| {
            format!(
                "{}/{}",
                p.os.clone().unwrap_or_default(),
                p.architecture.clone().unwrap_or_default()
            )
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("Platform      : {}\n", platform);

    let mut health_status: HealthStatusEnum = HealthStatusEnum::NONE;

    match state.health {
        Some(data) => {
            health_status = data.status.unwrap_or(HealthStatusEnum::NONE);
        }
        None => {}
    };

    //

    println!("State");
    println!("-----");

    println!("Status        : {:?}", state.status.unwrap());
    println!("Health        : {:?}", health_status);
    println!("PID           : {}", state.pid.unwrap_or_default());
    println!("Started At    : {}", state.started_at.unwrap_or_default());
    println!("OOM Killed    : {}", state.oom_killed.unwrap_or(false));
    println!("Restarting    : {}", state.restarting.unwrap_or(false));
    println!("Exit Code     : {}\n", state.exit_code.unwrap_or_default());

    println!("Process");
    println!("-------");

    println!("Entrypoint    : {}", container.path.unwrap_or_default());

    if let Some(args) = container.args {
        println!("Command       : {}", args.join(" "));
    }

    println!("\nNetwork");
    println!("-------");

    if let Some(net_settings) = network_settings {
        if let Some(networks) = net_settings.networks {
            for (name, net) in networks {
                println!("Network       : {}", name);
                println!("IP Address    : {}", net.ip_address.unwrap_or_default());
                println!("Gateway       : {}", net.gateway.unwrap_or_default());
            }
        }
    }

    println!("\nPorts");
    println!("-----");

    if let Some(host) = host_config {
        if let Some(bindings) = host.port_bindings {
            for (container_port, host_binding) in bindings {
                if let Some(binding) = host_binding {
                    for b in binding {
                        println!(
                            "{} -> {}:{}",
                            container_port,
                            b.host_ip.unwrap_or_default(),
                            b.host_port.unwrap_or_default()
                        );
                    }
                }
            }
        }

        println!("\nRestart Policy");
        println!("--------------");

        if let Some(policy) = host.restart_policy {
            println!(
                "Policy        : {}",
                policy.name.unwrap_or(RestartPolicyNameEnum::EMPTY)
            );
        }
    }

    println!(
        "Restart Count : {}",
        container.restart_count.unwrap_or_default()
    );

    Ok(true)
}
