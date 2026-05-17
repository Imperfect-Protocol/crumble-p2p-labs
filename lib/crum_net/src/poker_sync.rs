//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_pkr::{poker_hand::PokerHand, poker_table::PokerTable};

use crate::{P2pMessage, P2pTransport, PeerId, PokerAction, PokerActionEnvelope, PokerClientState};

pub struct PokerSyncClient<TTransport> {
    pub peers: Vec<PeerId>,
    pub client: PokerClientState,
    pub transport: TTransport,
}

impl<TTransport> PokerSyncClient<TTransport>
where
    TTransport: P2pTransport,
{
    pub fn new(
        player_index: usize,
        table_id: u64,
        hand_id: u64,
        peers: Vec<PeerId>,
        table: PokerTable,
        transport: TTransport,
    ) -> Self {
        Self {
            peers,
            client: PokerClientState::new(player_index, table_id, hand_id, table),
            transport,
        }
    }

    pub fn submit_local_action(&mut self, action: PokerAction) -> Result<(), Vec<u8>> {
        let envelope = self.client.submit_local_action(action)?;
        self.broadcast(envelope).map_err(|err| err.into_bytes())
    }

    pub fn poll_and_apply(&mut self) -> Result<usize, Vec<u8>> {
        let mut applied = 0;

        while let Some((_from_peer, message)) = self.transport.poll() {
            if self.apply_message(message)? {
                applied += 1;
            }
        }

        Ok(applied)
    }

    pub fn apply_message(&mut self, message: P2pMessage) -> Result<bool, Vec<u8>> {
        let P2pMessage::PokerAction(envelope) = message else {
            return Ok(false);
        };

        self.client.apply_envelope(envelope)
    }

    pub fn current_hand(&self) -> Option<&PokerHand> {
        self.client.current_hand()
    }

    fn broadcast(&mut self, envelope: PokerActionEnvelope) -> Result<(), String> {
        for peer in self.peers.clone() {
            self.transport
                .send(&peer, P2pMessage::PokerAction(envelope.clone()))?;
        }
        Ok(())
    }
}
