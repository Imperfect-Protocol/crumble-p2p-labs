//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::{
    NetworkConnector, PlayerEvent, PlayerIndex, PokerAction, PokerActionEnvelopeWire,
    PokerBotLogic, PokerBotLogicError, PokerClientState, PokerGameClientState, PokerGameState,
    PokerLobby, Sequence,
};
use crum_pkr::poker_table::PokerTable;

use crate::chain::HandStartedOnChain;

#[derive(Debug, Default)]
pub struct PokerGameConnector {
    connected: bool,
}

impl NetworkConnector for PokerGameConnector {}

impl PokerGameConnector {
    pub fn mark_connected(&mut self) {
        self.connected = true;
    }

    pub const fn is_connected(&self) -> bool {
        self.connected
    }
}

pub struct PokerGameSession<Connector = PokerGameConnector>
where
    Connector: NetworkConnector,
{
    pub state: PokerClientState,
    pub connector: Connector,
}

impl<Connector> PokerGameSession<Connector>
where
    Connector: NetworkConnector,
{
    pub fn new(state: PokerClientState, connector: Connector) -> Self {
        Self { state, connector }
    }
}

impl<Connector> crum_net::PokerGameSession for PokerGameSession<Connector>
where
    Connector: NetworkConnector,
{
    type Connector = Connector;

    fn state(&self) -> &PokerClientState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut PokerClientState {
        &mut self.state
    }

    fn connector(&self) -> &Self::Connector {
        &self.connector
    }

    fn connector_mut(&mut self) -> &mut Self::Connector {
        &mut self.connector
    }
}

pub struct PokerBotGame {
    session: PokerGameSession,
    lobby: PokerArbitrumGameLobby,
    logic: PokerBotLogic,
    sent_connection_info: bool,
    called_start_hand: bool,
    pending_ring_action: Option<(PlayerIndex, Sequence)>,
    playing: bool,
}

impl PokerBotGame {
    pub fn new(session: PokerGameSession, lobby: PokerArbitrumGameLobby) -> Self {
        Self {
            session,
            lobby,
            logic: PokerBotLogic::new(),
            sent_connection_info: false,
            called_start_hand: false,
            pending_ring_action: None,
            playing: false,
        }
    }

    pub fn for_arbitrum(
        state: PokerGameClientState,
        lobby_id: u64,
        table_id: u64,
        player_index: usize,
    ) -> Self {
        let mut game = Self::new(
            PokerGameSession::new(state, PokerGameConnector::default()),
            PokerArbitrumGameLobby { lobby_id, table_id },
        );
        game.logic.seat_as(player_index);
        game
    }

    pub fn session(&self) -> &PokerGameSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut PokerGameSession {
        &mut self.session
    }

    pub fn player_index(&self) -> usize {
        self.session.state.player_index
    }

    pub fn client_state(&self) -> &PokerClientState {
        &self.session.state
    }

    pub fn client_state_mut(&mut self) -> &mut PokerClientState {
        &mut self.session.state
    }

    pub fn lobby(&self) -> &PokerArbitrumGameLobby {
        &self.lobby
    }

    pub fn set_lobby(&mut self, lobby: PokerArbitrumGameLobby) {
        self.lobby = lobby;
    }

    pub fn logic_mut(&mut self) -> &mut PokerBotLogic {
        &mut self.logic
    }

    pub fn act(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), String> {
        let mut state = PokerBotGameState::new(&mut self.session.state);
        self.logic
            .act(&mut state, events)
            .map_err(|err| err.to_string())?;
        let (wire, log_lines) = state.into_wire()?;
        self.pending_ring_action = Some((wire.from, wire.sequence));
        Ok((wire, log_lines))
    }

    pub fn should_publish_connection_info(&self) -> bool {
        !self.sent_connection_info
    }

    pub fn mark_connection_info_sent(&mut self) {
        self.sent_connection_info = true;
    }

    pub fn mark_connected(&mut self) -> bool {
        if self.called_start_hand {
            return false;
        }
        self.called_start_hand = true;
        self.session.connector.mark_connected();
        true
    }

    pub fn on_hand_started(&mut self, event: &HandStartedOnChain) -> bool {
        if event.remain_count == alloy_primitives::U256::ZERO && !self.playing {
            self.playing = true;
            true
        } else {
            false
        }
    }

    pub fn should_tick(&self) -> bool {
        self.playing
    }

    pub fn has_pending_ring_action(&self) -> bool {
        self.pending_ring_action.is_some()
    }

    pub fn observe_ring_action(&mut self, wire: &PokerActionEnvelopeWire) {
        if self.pending_ring_action == Some((wire.from, wire.sequence)) {
            self.pending_ring_action = None;
        }
    }

    pub fn should_act(&self, acting_player: usize, player_index: usize, can_send: bool) -> bool {
        self.playing && acting_player == player_index && can_send
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PokerArbitrumGameLobby {
    pub lobby_id: u64,
    pub table_id: u64,
}

impl PokerLobby for PokerArbitrumGameLobby {
    type Error = String;

    async fn run(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl crum_net::PokerGame for PokerBotGame {
    type Session = PokerGameSession;

    fn session(&self) -> &Self::Session {
        &self.session
    }

    fn session_mut(&mut self) -> &mut Self::Session {
        &mut self.session
    }
}

pub struct PokerBotGameState<'state> {
    client_state: &'state mut PokerClientState,
    wire: Option<PokerActionEnvelopeWire>,
    log_lines: Vec<String>,
}

impl<'state> PokerBotGameState<'state> {
    fn new(client_state: &'state mut PokerClientState) -> Self {
        Self {
            client_state,
            wire: None,
            log_lines: Vec::new(),
        }
    }

    fn into_wire(self) -> Result<(PokerActionEnvelopeWire, Vec<String>), String> {
        let wire = self
            .wire
            .ok_or_else(|| "player did not submit an action".to_string())?;
        Ok((wire, self.log_lines))
    }
}

impl PokerGameState for PokerBotGameState<'_> {
    type Error = PokerBotLogicError;

    fn table(&self) -> &PokerTable {
        &self.client_state.table
    }

    fn submit_action(
        &mut self,
        action: PokerAction,
        log_lines: Vec<String>,
    ) -> Result<(), Self::Error> {
        let envelope = self.client_state.submit_local_action(action)?;
        self.wire = Some(envelope.to_wire(log_lines.clone()));
        self.log_lines = log_lines;
        Ok(())
    }
}
