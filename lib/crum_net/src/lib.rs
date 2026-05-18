//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

pub mod design;
pub mod messages;
pub mod network_model;
pub mod poker_bot_logic;
pub mod poker_deck_serialization;
pub mod poker_sync;
pub mod sync_stream;
pub mod topology;
pub mod transport;

pub use design::{
    LobbyCommand, LobbyId, NetworkEvent, NetworkRole, NetworkRunRequest, Player, PlayerAct,
    PlayerCommand, PlayerContext, PlayerEvent, PokerGameEvent, PokerGameState,
    PokerPlayerRegistrar, SignalBundle, TableHandle, TableSpec, create_table_command,
    handle_action_command, join_table_command, run_network_command, start_hand_command,
};
pub use messages::{
    EndpointDescriptor, HandId, HandshakeSignal, P2pMessage, PeerAnnouncement, PeerId, PlayerIndex,
    PokerAction, PokerActionEnvelope, PokerActionEnvelopeWire, PokerActionWire, PokerClientState,
    Sequence, TableId,
};
pub use network_model::{
    Bot, BotGame, Network, NetworkConnector, PokerGame, PokerGameSession, PokerLobby,
};
pub use poker_bot_logic::{
    HAND_ID, INITIAL_CHIPS, NUM_PLAYERS, PokerBotLogic, PokerBotLogicError, SMALL_BLIND, TABLE_ID,
    current_player, final_lines, is_finished, new_table, new_table_with_players,
};
pub use poker_sync::PokerSyncClient;
pub use sync_stream::{
    OrderedSyncState, SyncFrame, SyncFrameKind, SyncSeq, SyncStreamConfig, SyncStreamError,
    decode_frame_line, decode_frame_lines, encode_frame_line, encode_frame_lines,
};
pub use topology::{SeatedPeer, TopologyPolicy, expected_peers, outbound_offer_targets};
pub use transport::{InboundP2p, MemoryTransport, OutboundP2p, P2pTransport};

pub type PokerGameClientState = PokerClientState;
