//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::{PokerClientState, PokerNetwork, PokerPlayerRegistrar, TopologyPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokerPeerToPeerConnector {
    pub stun_server: String,
    pub player_details: PokerPeerToPeerPlayerDetails,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokerPeerToPeerPlayerDetails {
    pub annonce_public_key_hex: String,
}

impl PokerPeerToPeerConnector {
    pub fn new(
        stun_server: impl Into<String>,
        player_details: PokerPeerToPeerPlayerDetails,
    ) -> Self {
        Self {
            stun_server: stun_server.into(),
            player_details,
        }
    }
}

pub trait PokerPeerToPeerDetails {
    fn annonce_public_key_hex(&self) -> &str;
}

pub enum PokerPeerToPeerNetwork<'player> {
    NetworkConnector(PokerPeerToPeerRuntime<'player>),
}

pub struct PokerPeerToPeerRuntime<'player> {
    connector: PokerPeerToPeerConnector,
    topology: TopologyPolicy,
    expected_players: Option<u8>,
    table_ready: bool,
    player_registered: bool,
    _player_lifetime: std::marker::PhantomData<&'player mut ()>,
}

impl<'player> PokerPeerToPeerNetwork<'player> {
    pub fn use_connector(connector: PokerPeerToPeerConnector) -> Self {
        Self::NetworkConnector(PokerPeerToPeerRuntime {
            connector,
            topology: TopologyPolicy::Ring,
            expected_players: None,
            table_ready: false,
            player_registered: false,
            _player_lifetime: std::marker::PhantomData,
        })
    }

    pub fn with_topology(mut self, topology: TopologyPolicy, expected_players: Option<u8>) -> Self {
        let runtime = self.runtime_mut();
        runtime.topology = topology;
        runtime.expected_players = expected_players;
        self
    }

    pub fn mark_table_ready(&mut self) {
        self.runtime_mut().table_ready = true;
    }

    pub fn register_player<P>(&mut self, _player: &'player mut P) -> Result<(), String> {
        let runtime = self.runtime_mut();
        if runtime.player_registered {
            return Err("p2p network already has a registered player".to_string());
        }
        runtime.player_registered = true;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), String> {
        let runtime = self.runtime_mut();
        tracing::info!(
            "p2p network adapter ready: stun_server={}, topology={:?}, expected_players={:?}, table_ready={}, player_registered={}",
            runtime.connector.stun_server,
            runtime.topology,
            runtime.expected_players,
            runtime.table_ready,
            runtime.player_registered
        );
        tracing::info!(
            "WebRTC/STUN stream handling is the next adapter step; shared action/state logic lives in crum_net"
        );
        Ok(())
    }

    fn runtime_mut(&mut self) -> &mut PokerPeerToPeerRuntime<'player> {
        match self {
            Self::NetworkConnector(runtime) => runtime,
        }
    }

    fn runtime(&self) -> &PokerPeerToPeerRuntime<'player> {
        match self {
            Self::NetworkConnector(runtime) => runtime,
        }
    }
}

impl PokerNetwork for PokerPeerToPeerNetwork<'_> {
    type Error = String;

    fn install_poker_state(&mut self, _state: PokerClientState) -> Result<(), Self::Error> {
        Ok(())
    }

    fn mark_table_ready(&mut self) -> Result<(), Self::Error> {
        PokerPeerToPeerNetwork::mark_table_ready(self);
        Ok(())
    }
}

impl<'player, P> PokerPlayerRegistrar<'player, P> for PokerPeerToPeerNetwork<'player> {
    type Error = String;

    fn register_player(&mut self, player: &'player mut P) -> Result<(), Self::Error> {
        PokerPeerToPeerNetwork::register_player(self, player)
    }
}

impl PokerPeerToPeerDetails for PokerPeerToPeerNetwork<'_> {
    fn annonce_public_key_hex(&self) -> &str {
        &self
            .runtime()
            .connector
            .player_details
            .annonce_public_key_hex
    }
}
