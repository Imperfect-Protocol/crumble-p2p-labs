//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{error::Error, fmt, net::SocketAddr};

use crum_net::{
    PlayerEvent, PokerActionEnvelopeWire, PokerBotLogicError, PokerPlayerRegistrar, current_player,
    final_lines, is_finished,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        TcpListener, TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use crate::PokerBot;

#[derive(Debug)]
pub struct PokerServer {
    host: String,
    port: u16,
}

impl PokerServer {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}

#[derive(Debug)]
pub struct PokerClient {
    host: String,
    port: u16,
}

impl PokerClient {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}

#[derive(Debug)]
enum PokerEndpoint {
    Server(PokerServer),
    Client(PokerClient),
}

pub enum PokerClientServerNetwork<'player> {
    NetworkServer(PokerClientServerRuntime<'player>),
    NetworkClient(PokerClientServerRuntime<'player>),
}

pub struct PokerClientServerRuntime<'player> {
    endpoint: PokerEndpoint,
    player: Option<&'player mut PokerBot>,
    reader: Option<BufReader<OwnedReadHalf>>,
    writer: Option<OwnedWriteHalf>,
}

impl<'player> PokerClientServerNetwork<'player> {
    pub async fn use_server(server: PokerServer) -> Result<Self, NetworkError> {
        let mut network = PokerClientServerRuntime {
            endpoint: PokerEndpoint::Server(server),
            player: None,
            reader: None,
            writer: None,
        };
        network.connect().await?;
        Ok(Self::NetworkServer(network))
    }

    pub async fn use_client(client: PokerClient) -> Result<Self, NetworkError> {
        let mut network = PokerClientServerRuntime {
            endpoint: PokerEndpoint::Client(client),
            player: None,
            reader: None,
            writer: None,
        };
        network.connect().await?;
        Ok(Self::NetworkClient(network))
    }

    pub fn register_player(&mut self, player: &'player mut PokerBot) -> Result<(), NetworkError> {
        self.runtime_mut().register_player(player)
    }

    pub async fn run(&mut self) -> Result<(), NetworkError> {
        self.runtime_mut().run().await
    }

    fn runtime_mut(&mut self) -> &mut PokerClientServerRuntime<'player> {
        match self {
            Self::NetworkServer(runtime) | Self::NetworkClient(runtime) => runtime,
        }
    }
}

impl<'player> PokerPlayerRegistrar<'player, PokerBot> for PokerClientServerNetwork<'player> {
    type Error = NetworkError;

    fn register_player(&mut self, player: &'player mut PokerBot) -> Result<(), Self::Error> {
        PokerClientServerNetwork::register_player(self, player)
    }
}

impl<'player> PokerClientServerRuntime<'player> {
    pub fn register_player(&mut self, player: &'player mut PokerBot) -> Result<(), NetworkError> {
        if self.player.is_some() {
            return Err(NetworkError::from(
                "network already has a registered player",
            ));
        }
        self.player = Some(player);
        Ok(())
    }

    async fn connect(&mut self) -> Result<(), NetworkError> {
        if self.reader.is_some() && self.writer.is_some() {
            return Ok(());
        }

        let stream = match &self.endpoint {
            PokerEndpoint::Server(server) => {
                let addr = socket_addr(&server.host, server.port)?;
                let listener = TcpListener::bind(addr).await?;
                tracing::info!("Player 1 listening on {addr}");
                let (stream, peer) = listener.accept().await?;
                tracing::info!("Player 1 accepted peer {peer}");
                stream
            }
            PokerEndpoint::Client(client) => {
                let addr = socket_addr(&client.host, client.port)?;
                let stream = TcpStream::connect(addr).await?;
                tracing::info!("Player 2 connected to {addr}");
                stream
            }
        };

        let (reader, writer) = stream.into_split();
        self.reader = Some(BufReader::new(reader));
        self.writer = Some(writer);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), NetworkError> {
        self.connect().await?;
        let player_index = self.player()?.player_index()?;
        tracing::info!("Starting gameplay as Player {}", player_index + 1);

        loop {
            if is_finished(&self.player()?.client_state()?.table)? {
                for line in final_lines(&self.player()?.client_state()?.table)? {
                    println!("{line}");
                }
                break;
            }

            let acting_player = current_player(&self.player()?.client_state()?.table)?;
            if acting_player == player_index {
                let (wire, log_lines) = self.call_player_act(&[PlayerEvent::Tick])?;
                for line in &log_lines {
                    println!("{line}");
                }
                self.send_action(wire).await?;
            } else {
                let wire = self.receive_action().await?;
                let envelope = wire.to_envelope().map_err(NetworkError::from)?;
                self.player_mut()?
                    .client_state_mut()?
                    .apply_envelope(envelope)?;
                for line in wire.log_lines {
                    println!("{line}");
                }
            }
        }

        Ok(())
    }

    pub fn player(&self) -> Result<&PokerBot, NetworkError> {
        self.player
            .as_deref()
            .ok_or_else(|| NetworkError::from("network has no registered player"))
    }

    pub fn player_mut(&mut self) -> Result<&mut PokerBot, NetworkError> {
        self.player
            .as_deref_mut()
            .ok_or_else(|| NetworkError::from("network has no registered player"))
    }

    pub async fn send_action(&mut self, wire: PokerActionEnvelopeWire) -> Result<(), NetworkError> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| NetworkError::from("network has not been run"))?;
        let mut encoded = serde_json::to_string(&wire)?;
        encoded.push('\n');
        writer.write_all(encoded.as_bytes()).await?;
        Ok(())
    }

    pub async fn receive_action(&mut self) -> Result<PokerActionEnvelopeWire, NetworkError> {
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| NetworkError::from("network has not been run"))?;
        let mut inbound = String::new();
        let bytes_read = reader.read_line(&mut inbound).await?;
        if bytes_read == 0 {
            return Err(NetworkError::from("peer disconnected before hand finished"));
        }

        Ok(serde_json::from_str(inbound.trim_end())?)
    }

    fn call_player_act(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), NetworkError> {
        self.player_mut()?.act_current_game(events)
    }
}

#[derive(Debug)]
pub struct NetworkError(String);

impl fmt::Display for NetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for NetworkError {}

impl From<String> for NetworkError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for NetworkError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<Vec<u8>> for NetworkError {
    fn from(value: Vec<u8>) -> Self {
        Self(
            String::from_utf8(value)
                .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned()),
        )
    }
}

impl From<PokerBotLogicError> for NetworkError {
    fn from(value: PokerBotLogicError) -> Self {
        Self(value.to_string())
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(value: serde_json::Error) -> Self {
        Self(value.to_string())
    }
}

fn socket_addr(host: &str, port: u16) -> Result<SocketAddr, NetworkError> {
    format!("{host}:{port}")
        .parse()
        .map_err(|err| NetworkError::from(format!("invalid socket address {host}:{port}: {err}")))
}
