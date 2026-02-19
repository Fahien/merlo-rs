// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_replicon::prelude::RepliconChannels;
use bevy_replicon_renet::{
    RenetChannelsExt,
    netcode::{
        ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
        ServerConfig,
    },
    renet::{ConnectionConfig, RenetClient, RenetServer},
};
use clap::Parser;

const DEFAULT_PORT: u16 = 5000;
const PROTOCOL_ID: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Singleplayer,
    Server,
    Client,
}

/// An RTS demo.
#[derive(Parser, PartialEq, Resource)]
pub enum Cli {
    /// Play locally.
    Singleplayer {},
    /// Create a server that acts as both player and host.
    Server {
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
    /// Connect to a host.
    Client {
        #[arg(short, long, default_value_t = Ipv4Addr::LOCALHOST.into())]
        ip: IpAddr,

        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}

pub fn init(
    commands: &mut Commands,
    cli: &Cli,
    channels: &RepliconChannels,
) -> Result<NetworkMode> {
    match *cli {
        Cli::Singleplayer {} => Ok(NetworkMode::Singleplayer),
        Cli::Server { port } => {
            init_server(commands, channels, port)?;
            Ok(NetworkMode::Server)
        }
        Cli::Client { ip, port } => {
            init_client(commands, channels, ip, port)?;
            Ok(NetworkMode::Client)
        }
    }
}

fn init_server(commands: &mut Commands, channels: &RepliconChannels, port: u16) -> Result<()> {
    let server = RenetServer::new(connection_config(channels));

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?;
    let server_config = ServerConfig {
        current_time,
        max_clients: 1,
        protocol_id: PROTOCOL_ID,
        authentication: ServerAuthentication::Unsecure,
        public_addresses: Default::default(),
    };
    let transport = NetcodeServerTransport::new(server_config, socket)?;

    commands.insert_resource(server);
    commands.insert_resource(transport);
    commands.spawn(Text::new("Server"));

    Ok(())
}

fn init_client(
    commands: &mut Commands,
    channels: &RepliconChannels,
    ip: IpAddr,
    port: u16,
) -> Result<()> {
    info!("connecting to {ip}:{port}");

    let client = RenetClient::new(connection_config(channels));

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let server_addr = SocketAddr::new(ip, port);
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let addr = socket.local_addr()?;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: None,
    };
    let transport = NetcodeClientTransport::new(current_time, authentication, socket)?;

    commands.insert_resource(client);
    commands.insert_resource(transport);
    commands.spawn(Text(format!("Client: {addr}")));

    Ok(())
}

fn connection_config(channels: &RepliconChannels) -> ConnectionConfig {
    ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..Default::default()
    }
}
