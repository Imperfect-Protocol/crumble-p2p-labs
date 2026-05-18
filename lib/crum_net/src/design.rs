//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use serde::{Deserialize, Serialize};

use crum_pkr::poker_table::PokerTable;

use crate::{
    HandId, HandshakeSignal, PeerAnnouncement, PeerId, PlayerIndex, PokerAction,
    PokerActionEnvelopeWire, PokerClientState, SeatedPeer, SyncFrame, TableId, TopologyPolicy,
};

pub type LobbyId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkRole {
    ClientServer,
    PeerToPeer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableSpec {
    pub lobby_id: LobbyId,
    pub topology: TopologyPolicy,
    pub min_players: usize,
    pub max_players: usize,
    pub buy_in_chips: u64,
    pub metadata: Option<String>,
}

impl TableSpec {
    pub fn new(lobby_id: LobbyId, topology: TopologyPolicy) -> Self {
        Self {
            lobby_id,
            topology,
            min_players: 2,
            max_players: 9,
            buy_in_chips: 0,
            metadata: None,
        }
    }

    pub fn with_player_limits(mut self, min_players: usize, max_players: usize) -> Self {
        self.min_players = min_players;
        self.max_players = max_players;
        self
    }

    pub fn with_buy_in_chips(mut self, buy_in_chips: u64) -> Self {
        self.buy_in_chips = buy_in_chips;
        self
    }

    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableHandle {
    pub lobby_id: LobbyId,
    pub table_id: TableId,
    pub topology: TopologyPolicy,
    pub seated: Vec<SeatedPeer>,
    pub hand_id: Option<HandId>,
}

impl TableHandle {
    pub fn new(lobby_id: LobbyId, table_id: TableId, topology: TopologyPolicy) -> Self {
        Self {
            lobby_id,
            table_id,
            topology,
            seated: Vec::new(),
            hand_id: None,
        }
    }

    pub fn with_seated(mut self, seated: Vec<SeatedPeer>) -> Self {
        self.seated = seated;
        self
    }

    pub fn with_hand_id(mut self, hand_id: HandId) -> Self {
        self.hand_id = Some(hand_id);
        self
    }

    pub fn seat(&self, player_index: PlayerIndex) -> Option<&SeatedPeer> {
        self.seated
            .iter()
            .find(|peer| peer.player_index == player_index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalBundle {
    pub table_id: TableId,
    pub hand_id: Option<HandId>,
    pub from: PeerId,
    pub recipients: Vec<PeerId>,
    pub encrypted_data: Vec<Vec<u8>>,
}

impl SignalBundle {
    pub fn new(
        table_id: TableId,
        from: impl Into<PeerId>,
        recipients: Vec<PeerId>,
        encrypted_data: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            table_id,
            hand_id: None,
            from: from.into(),
            recipients,
            encrypted_data,
        }
    }

    pub fn with_hand_id(mut self, hand_id: HandId) -> Self {
        self.hand_id = Some(hand_id);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRunRequest {
    pub table: TableHandle,
    pub role: NetworkRole,
    pub local_peer_id: PeerId,
    pub local_player_index: PlayerIndex,
}

impl NetworkRunRequest {
    pub fn new(
        table: TableHandle,
        role: NetworkRole,
        local_peer_id: impl Into<PeerId>,
        local_player_index: PlayerIndex,
    ) -> Self {
        Self {
            table,
            role,
            local_peer_id: local_peer_id.into(),
            local_player_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LobbyCommand {
    CreateTable {
        spec: TableSpec,
        annonce_public_key: Vec<u8>,
    },
    JoinTable {
        table_id: TableId,
        annonce_public_key: Vec<u8>,
    },
    LeaveTable {
        table_id: TableId,
    },
    StartHand {
        table_id: TableId,
    },
    SendSignal(SignalBundle),
    RunNetwork(NetworkRunRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkEvent {
    TableCreated {
        table: TableHandle,
    },
    TableJoined {
        table: TableHandle,
        player_index: PlayerIndex,
    },
    PlayerAnnounced(PeerAnnouncement),
    SignalReceived(HandshakeSignal),
    SignalPublished(SignalBundle),
    HandStarted {
        table_id: TableId,
        hand_id: HandId,
    },
    PokerActionReceived(PokerActionEnvelopeWire),
    PokerActionBroadcast(PokerActionEnvelopeWire),
    SyncFrameReceived(SyncFrame),
    NetworkReady {
        table_id: TableId,
        connected_peers: Vec<PeerId>,
    },
    NetworkStopped {
        table_id: TableId,
    },
    Error {
        message: String,
    },
}

#[derive(Clone)]
pub enum PlayerEvent {
    CommandAccepted(LobbyCommand),
    Network(NetworkEvent),
    LocalActionRequested(PokerAction),
    Tick,
}

pub trait PokerGameEvent: Clone {}

impl PokerGameEvent for PlayerEvent {}

pub struct PlayerContext {
    pub role: NetworkRole,
    pub local_peer_id: PeerId,
    pub local_player_index: Option<PlayerIndex>,
    pub table: Option<TableHandle>,
    pub poker: Option<PokerClientState>,
}

impl PlayerContext {
    pub fn new(role: NetworkRole, local_peer_id: impl Into<PeerId>) -> Self {
        Self {
            role,
            local_peer_id: local_peer_id.into(),
            local_player_index: None,
            table: None,
            poker: None,
        }
    }

    pub fn for_table(
        role: NetworkRole,
        local_peer_id: impl Into<PeerId>,
        local_player_index: PlayerIndex,
        table: TableHandle,
    ) -> Self {
        Self {
            role,
            local_peer_id: local_peer_id.into(),
            local_player_index: Some(local_player_index),
            table: Some(table),
            poker: None,
        }
    }

    pub fn attach_poker_state(&mut self, poker: PokerClientState) {
        self.poker = Some(poker);
    }

    pub fn set_table(&mut self, table: TableHandle) {
        self.local_player_index = table
            .seated
            .iter()
            .find(|peer| peer.peer_id == self.local_peer_id)
            .map(|peer| peer.player_index)
            .or(self.local_player_index);
        self.table = Some(table);
    }

    pub fn table_id(&self) -> Option<TableId> {
        self.table.as_ref().map(|table| table.table_id)
    }
}

#[derive(Clone)]
pub enum PlayerCommand {
    Lobby(LobbyCommand),
    SubmitPokerAction(PokerAction),
    BroadcastPokerAction(PokerActionEnvelopeWire),
    SendPokerAction {
        peer_id: PeerId,
        envelope: PokerActionEnvelopeWire,
    },
    EmitSyncFrame(SyncFrame),
}

#[derive(Clone, Default)]
pub struct PlayerAct {
    pub commands: Vec<PlayerCommand>,
}

impl PlayerAct {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn single(command: PlayerCommand) -> Self {
        Self {
            commands: vec![command],
        }
    }

    pub fn push(&mut self, command: PlayerCommand) {
        self.commands.push(command);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

pub trait PokerGameState {
    type Error;

    fn table(&self) -> &PokerTable;

    fn submit_action(
        &mut self,
        action: PokerAction,
        log_lines: Vec<String>,
    ) -> Result<(), Self::Error>;
}

pub trait Player<State, Event>
where
    State: PokerGameState,
    Event: PokerGameEvent,
{
    type Error;

    fn act(&mut self, state: &mut State, events: &[Event]) -> Result<(), Self::Error>;
}

pub trait PokerPlayerRegistrar<'player, P> {
    type Error;

    fn register_player(&mut self, player: &'player mut P) -> Result<(), Self::Error>;
}

pub fn create_table_command(spec: TableSpec, annonce_public_key: Vec<u8>) -> LobbyCommand {
    LobbyCommand::CreateTable {
        spec,
        annonce_public_key,
    }
}

pub fn join_table_command(table_id: TableId, annonce_public_key: Vec<u8>) -> LobbyCommand {
    LobbyCommand::JoinTable {
        table_id,
        annonce_public_key,
    }
}

pub fn start_hand_command(table_id: TableId) -> LobbyCommand {
    LobbyCommand::StartHand { table_id }
}

pub fn run_network_command(request: NetworkRunRequest) -> LobbyCommand {
    LobbyCommand::RunNetwork(request)
}

pub fn handle_action_command(envelope: PokerActionEnvelopeWire) -> PlayerCommand {
    PlayerCommand::BroadcastPokerAction(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> TableHandle {
        TableHandle::new(7, 42, TopologyPolicy::Ring).with_seated(vec![
            SeatedPeer {
                player_index: 0,
                peer_id: "alice".to_string(),
            },
            SeatedPeer {
                player_index: 1,
                peer_id: "bob".to_string(),
            },
        ])
    }

    #[test]
    fn context_sets_local_player_from_table() {
        let mut context = PlayerContext::new(NetworkRole::PeerToPeer, "bob");

        context.set_table(table());

        assert_eq!(context.table_id(), Some(42));
        assert_eq!(context.local_player_index, Some(1));
    }

    #[test]
    fn helper_commands_describe_table_lifecycle() {
        let spec = TableSpec::new(7, TopologyPolicy::Mesh).with_buy_in_chips(100);
        let create = create_table_command(spec.clone(), vec![1, 2, 3]);
        let join = join_table_command(42, vec![4, 5, 6]);

        assert_eq!(
            create,
            LobbyCommand::CreateTable {
                spec,
                annonce_public_key: vec![1, 2, 3],
            }
        );
        assert_eq!(
            join,
            LobbyCommand::JoinTable {
                table_id: 42,
                annonce_public_key: vec![4, 5, 6],
            }
        );
    }
}
