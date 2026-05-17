//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
};

use crate::P2pMessage;

pub trait OutboundP2p {
    fn send(&mut self, peer_id: &str, message: P2pMessage) -> Result<(), String>;
}

pub trait InboundP2p {
    fn poll(&mut self) -> Option<(String, P2pMessage)>;
}

pub trait P2pTransport: OutboundP2p + InboundP2p + Debug {}

impl<T> P2pTransport for T where T: OutboundP2p + InboundP2p + Debug {}

#[derive(Default)]
pub struct MemoryTransport {
    outbound: HashMap<String, Vec<P2pMessage>>,
    inbound: VecDeque<(String, P2pMessage)>,
}

impl Debug for MemoryTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MemoryTransport")
            .field("outbound_peer_count", &self.outbound.len())
            .field("inbound_len", &self.inbound.len())
            .finish()
    }
}

impl MemoryTransport {
    pub fn push_inbound(&mut self, from: impl Into<String>, message: P2pMessage) {
        self.inbound.push_back((from.into(), message));
    }

    pub fn drain_outbound_for(&mut self, peer_id: &str) -> Vec<P2pMessage> {
        self.outbound.remove(peer_id).unwrap_or_default()
    }
}

impl OutboundP2p for MemoryTransport {
    fn send(&mut self, peer_id: &str, message: P2pMessage) -> Result<(), String> {
        self.outbound
            .entry(peer_id.to_string())
            .or_default()
            .push(message);
        Ok(())
    }
}

impl InboundP2p for MemoryTransport {
    fn poll(&mut self) -> Option<(String, P2pMessage)> {
        self.inbound.pop_front()
    }
}
