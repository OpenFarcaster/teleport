use std::{net::IpAddr, option};

use libp2p::{multiaddr::Protocol, Multiaddr};

use crate::core::errors::{BadRequestType, HubError};

#[derive(Debug)]
pub enum IpVersion {
    Invalid,
    IPv4,
    IPv6,
}

pub fn parse_address(multiaddr_str: &str) -> Result<Multiaddr, HubError> {
    if multiaddr_str.is_empty() {
        return Err(HubError::BadRequest(
            BadRequestType::Generic,
            "multiaddr must not be empty".to_owned(),
        ));
    }

    let multiaddr = libp2p::multiaddr::from_url(multiaddr_str);
    match multiaddr {
        Err(_) => Err(HubError::BadRequest(
            BadRequestType::ParseFailure,
            format!("'{}': invalid multiaddr", multiaddr_str),
        )),
        Ok(multiaddr) => Ok(multiaddr),
    }
}

pub fn get_ip_version(input: &str) -> IpVersion {
    match input.parse::<IpAddr>() {
        Ok(ip_addr) => {
            if ip_addr.is_ipv4() {
                IpVersion::IPv4
            } else {
                IpVersion::IPv6
            }
        }
        Err(_) => IpVersion::Invalid,
    }
}

pub fn check_node_addrs(
    listen_ip_addr: String,
    listen_combined_addr: String,
) -> Result<(), HubError> {
    let result_ip = check_ip_addr(&listen_ip_addr);
    let result_combined = check_combined_addr(&listen_combined_addr);

    result_ip.and_then(|_| result_combined)
}

fn check_ip_addr(ip_addr: &str) -> Result<(), HubError> {
    let parsed_addr = parse_address(ip_addr);
    if parsed_addr.is_err() {
        return Err(parsed_addr.unwrap_err());
    }

    let binding = parsed_addr.unwrap();
    let options = binding.iter().collect::<Vec<_>>();
    if options.len() > 1 {
        return Err(HubError::BadRequest(
            BadRequestType::Generic,
            "unexpected multiaddr transport/port information".to_owned(),
        ));
    }

    Ok(())
}

fn check_combined_addr(combined_addr: &str) -> Result<(), HubError> {
    let parsed_addr = parse_address(combined_addr);

    if parsed_addr.is_err() {
        return Err(parsed_addr.unwrap_err());
    }

    let multi_addr = parsed_addr.unwrap();
    let components = multi_addr.iter().collect::<Vec<_>>();
    let protocol_option = components.get(1);

    if let Some(Protocol::Tcp(_)) = protocol_option {
        // Handle the case where the second element is of Protocol::Tcp variant
        println!("Second option is Tcp");
    } else {
        return Err(HubError::BadRequest(
            BadRequestType::Generic,
            "multiaddr transport must be tcp".to_owned(),
        ));
    }

    Ok(())
}
