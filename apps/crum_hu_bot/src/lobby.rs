//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::collections::HashMap;

use crum_net::{HAND_ID, LobbyId, PokerClientState, PokerNetwork, TABLE_ID, TableId, new_table};

use crate::network::NetworkError;

pub const DEFAULT_LOBBY_ID: LobbyId = 1;

pub struct ClientServerLobbyRecord {
    #[allow(dead_code)]
    pub lobby_id: LobbyId,
    #[allow(dead_code)]
    pub name: String,
    pub tables: HashMap<TableId, ClientServerTableRecord>,
}

pub struct ClientServerTableRecord {
    #[allow(dead_code)]
    pub table_id: TableId,
    #[allow(dead_code)]
    pub name: String,
}

pub struct PokerClientServerLobby<N>
where
    N: PokerNetwork<Error = NetworkError>,
{
    network: N,
    lobbies: HashMap<LobbyId, ClientServerLobbyRecord>,
}

impl<N> PokerClientServerLobby<N>
where
    N: PokerNetwork<Error = NetworkError>,
{
    pub fn use_network(network: N) -> Self {
        let mut lobby = Self {
            network,
            lobbies: HashMap::new(),
        };
        lobby.ensure_lobby(DEFAULT_LOBBY_ID, "Heads-Up Demo");
        lobby
    }

    pub fn create_table(&mut self, player_index: usize) -> Result<(), NetworkError> {
        self.put_table(DEFAULT_LOBBY_ID, TABLE_ID, "Heads-Up Demo Table");
        self.use_table_state(player_index)
    }

    pub fn join_table(&mut self, player_index: usize) -> Result<(), NetworkError> {
        if self.table(DEFAULT_LOBBY_ID, TABLE_ID).is_none() {
            self.put_table(DEFAULT_LOBBY_ID, TABLE_ID, "Heads-Up Demo Table");
        }
        self.use_table_state(player_index)
    }

    pub fn into_network(self) -> N {
        self.network
    }

    #[allow(dead_code)]
    pub fn lobby(&self, lobby_id: LobbyId) -> Option<&ClientServerLobbyRecord> {
        self.lobbies.get(&lobby_id)
    }

    pub fn table(&self, lobby_id: LobbyId, table_id: TableId) -> Option<&ClientServerTableRecord> {
        self.lobbies
            .get(&lobby_id)
            .and_then(|lobby| lobby.tables.get(&table_id))
    }

    fn ensure_lobby(&mut self, lobby_id: LobbyId, name: &str) {
        self.lobbies
            .entry(lobby_id)
            .or_insert_with(|| ClientServerLobbyRecord {
                lobby_id,
                name: name.to_string(),
                tables: HashMap::new(),
            });
    }

    fn put_table(&mut self, lobby_id: LobbyId, table_id: TableId, name: &str) {
        self.ensure_lobby(lobby_id, "Heads-Up Demo");
        if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
            lobby.tables.insert(
                table_id,
                ClientServerTableRecord {
                    table_id,
                    name: name.to_string(),
                },
            );
        }
    }

    fn use_table_state(&mut self, player_index: usize) -> Result<(), NetworkError> {
        self.network.install_poker_state(PokerClientState::new(
            player_index,
            TABLE_ID,
            HAND_ID,
            new_table()?,
        ))?;
        self.network.mark_table_ready()
    }
}
