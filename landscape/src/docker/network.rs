use std::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
};

use bollard::{
    config::Ipam,
    models::{Network, NetworkInspect},
    query_parameters::ListNetworksOptions,
    Docker,
};
use landscape_common::docker::{
    network::{LandscapeDockerIpInfo, LandscapeDockerNetwork},
    DOCKER_NETWORK_BRIDGE_NAME_OPTION_KEY,
};

pub async fn inspect_all_networks() -> Vec<LandscapeDockerNetwork> {
    let docker = Docker::connect_with_socket_defaults();

    let Ok(docker) = docker else {
        tracing::warn!("Docker Connect Fail");
        return vec![];
    };

    let query: Option<ListNetworksOptions> = None;
    let networks = match docker.list_networks(query).await {
        Ok(networks) => networks,
        Err(err) => {
            tracing::warn!(error = %err, "Docker list_networks failed");
            return vec![];
        }
    };

    let mut result = Vec::with_capacity(networks.len());
    for network in networks {
        if let Some(net) = convert_network(network) {
            result.push(net);
        }
    }

    result
}

pub fn convert_network_inspect(net: NetworkInspect) -> Option<LandscapeDockerNetwork> {
    match (net.name, net.id) {
        (Some(name), Some(id)) => {
            let options = net.options.unwrap_or_default();

            let iface_name = if let Some(name) = options.get(DOCKER_NETWORK_BRIDGE_NAME_OPTION_KEY)
            {
                name.to_string()
            } else {
                format!("br-{}", id.get(..12).unwrap_or(&id))
            };

            let ip_info = net.ipam.map(convert_ipam).flatten();

            Some(LandscapeDockerNetwork {
                name,
                iface_name,
                id,
                options,
                driver: net.driver,
                ip_info,
            })
        }
        _ => None,
    }
}

pub fn convert_network(net: Network) -> Option<LandscapeDockerNetwork> {
    match (net.name, net.id) {
        (Some(name), Some(id)) => {
            let options = net.options.unwrap_or_default();

            let iface_name = if let Some(name) = options.get(DOCKER_NETWORK_BRIDGE_NAME_OPTION_KEY)
            {
                name.to_string()
            } else {
                format!("br-{}", id.get(..12).unwrap_or(&id))
            };

            let ip_info = net.ipam.map(convert_ipam).flatten();

            Some(LandscapeDockerNetwork {
                name,
                iface_name,
                id,
                options,
                driver: net.driver,
                ip_info,
            })
        }
        _ => None,
    }
}

fn convert_ipam(ipam: Ipam) -> Option<LandscapeDockerIpInfo> {
    let Some(config) = ipam.config.as_ref().map(|c| c.get(0)).flatten() else {
        return None;
    };

    let Some(subnet) = config.subnet.as_ref() else {
        return None;
    };
    let Ok(subnet) = cidr::Ipv4Inet::from_str(subnet) else {
        return None;
    };

    Some(LandscapeDockerIpInfo {
        subnet_ip: IpAddr::V4(subnet.address()),
        prefix: subnet.network_length(),
        gateway: IpAddr::V4(
            config
                .gateway
                .as_ref()
                .and_then(|gw| gw.parse::<Ipv4Addr>().ok())
                .unwrap_or_else(|| subnet.overflowing_add_u32(1).0.address()),
        ),
    })
}
