//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

pub mod design;
pub mod in_memory;
pub mod messages;
pub mod poker_bot;
pub mod poker_deck_serialization;
pub mod poker_sync;
pub mod sync_stream;
pub mod topology;
pub mod transport;

pub use design::{
    LobbyCommand, LobbyId, NetworkEvent, NetworkRole, NetworkRunRequest, Player, PlayerAct,
    PlayerCommand, PlayerContext, PlayerEvent, PokerGameEvent, PokerGameState, PokerNetwork,
    PokerPlayerRegistrar, SignalBundle, TableHandle, TableSpec, create_table_command,
    handle_action_command, join_table_command, run_network_command, start_hand_command,
};
pub use in_memory::{InMemoryNetwork, InMemoryPokerGameState};
pub use messages::{
    EndpointDescriptor, HandId, HandshakeSignal, P2pMessage, PeerAnnouncement, PeerId, PlayerIndex,
    PokerAction, PokerActionEnvelope, PokerActionEnvelopeWire, PokerActionWire, PokerClientState,
    Sequence, TableId,
};
pub use poker_bot::{
    HAND_ID, INITIAL_CHIPS, NUM_PLAYERS, PokerBot, PokerBotError, SMALL_BLIND, TABLE_ID,
    current_player, final_lines, is_finished, new_table,
};
pub use poker_sync::PokerSyncClient;
pub use sync_stream::{
    OrderedSyncState, SyncFrame, SyncFrameKind, SyncSeq, SyncStreamConfig, SyncStreamError,
    decode_frame_line, decode_frame_lines, encode_frame_line, encode_frame_lines,
};
pub use topology::{SeatedPeer, TopologyPolicy, expected_peers, outbound_offer_targets};
pub use transport::{InboundP2p, MemoryTransport, OutboundP2p, P2pTransport};
