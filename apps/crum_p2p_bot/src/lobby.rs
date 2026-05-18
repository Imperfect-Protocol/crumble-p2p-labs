//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::{HAND_ID, PokerClientState, new_table_with_players};

use crate::{
    bot::PokerBot,
    chain::{
        self, ChainConnector, PrivatePokerHandClient, PrivatePokerLobbyClient,
        PrivatePokerLobbyContract, PrivatePokerSignalClient, PrivatePokerSpectateClient,
        PrivatePokerSpectateContract, PrivatePokerTableClient, PrivatePokerTableContract,
    },
    network::{PeerToPeerNetwork, PokerPeerToPeerConnector, PokerPeerToPeerDetails},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokerArbitrumDetails {
    rpc_url: Option<String>,
    private_key: String,
    lobby_address: String,
    signal_address: Option<String>,
    lobby_id: u64,
}

impl PokerArbitrumDetails {
    pub fn new(
        rpc_url: Option<String>,
        private_key: impl Into<String>,
        lobby_address: impl Into<String>,
        lobby_id: u64,
    ) -> Self {
        Self {
            rpc_url,
            private_key: private_key.into(),
            lobby_address: lobby_address.into(),
            signal_address: std::env::var("PP_SIGNAL").ok(),
            lobby_id,
        }
    }

    pub fn with_signal_address(mut self, signal_address: Option<String>) -> Self {
        if signal_address.is_some() {
            self.signal_address = signal_address;
        }
        self
    }
}

pub struct PokerArbitrumLobby<N>
where
    N: PokerPeerToPeerDetails,
{
    network: N,
    details: PokerArbitrumDetails,
    connector: ChainConnector,
    lobby_client: PrivatePokerLobbyClient,
    table_client: PrivatePokerTableClient,
    hand_client: PrivatePokerHandClient,
    spectate_client: PrivatePokerSpectateClient,
    signal_client: PrivatePokerSignalClient,
}

impl<N> PokerArbitrumLobby<N>
where
    N: PokerPeerToPeerDetails,
{
    pub async fn use_network(network: N, details: PokerArbitrumDetails) -> Result<Self, String> {
        let connector =
            ChainConnector::connect(details.rpc_url.as_deref(), &details.private_key).await?;
        tracing::info!(
            "connected to Nitro/Arbitrum RPC: url={}, chain_id={}, wallet={}",
            connector.rpc_url,
            connector.chain_id,
            connector.wallet_address
        );
        let signal_address = details
            .signal_address
            .clone()
            .unwrap_or_else(|| details.lobby_address.clone());
        let lobby_client =
            PrivatePokerLobbyClient::new(connector.clone(), details.lobby_address.clone());
        let table_client =
            PrivatePokerTableClient::new(connector.clone(), details.lobby_address.clone());
        let hand_client =
            PrivatePokerHandClient::new(connector.clone(), details.lobby_address.clone());
        let spectate_client =
            PrivatePokerSpectateClient::new(connector.clone(), details.lobby_address.clone());
        let signal_client = PrivatePokerSignalClient::new(connector.clone(), signal_address);

        Ok(Self {
            network,
            details,
            connector,
            lobby_client,
            table_client,
            hand_client,
            spectate_client,
            signal_client,
        })
    }

    pub async fn create_table(
        &mut self,
        table_name: &str,
        buy_in: u64,
        num_players: u8,
    ) -> Result<chain::TableCreatedOnChain, String> {
        let created = self
            .lobby_client
            .create_table(
                self.details.lobby_id,
                table_name,
                buy_in,
                num_players,
                self.network.annonce_public_key_hex(),
            )
            .await?;
        self.network.prepare_table_creator();
        self.install_signal_context(created.table_id);
        Ok(created)
    }

    pub async fn join_table(&mut self, table_id: u64) -> Result<chain::TableJoinedOnChain, String> {
        let joined = self
            .table_client
            .join_table(
                self.details.lobby_id,
                table_id,
                self.network.annonce_public_key_hex(),
            )
            .await?;
        self.network.prepare_table_joiner();
        self.install_signal_context(joined.table_id);
        Ok(joined)
    }

    pub async fn find_table_by_name(
        &self,
        table_name: &str,
    ) -> Result<Option<chain::TableInfoOnChain>, String> {
        self.spectate_client
            .find_table_by_name(self.details.lobby_id, table_name)
            .await
    }

    pub fn lobby_id(&self) -> u64 {
        self.details.lobby_id
    }

    fn install_signal_context(&mut self, table_id: u64) {
        self.network.install_chain(
            self.table_client.clone(),
            self.hand_client.clone(),
            self.signal_client.clone(),
            self.connector.wallet_address,
            self.details.lobby_id,
            table_id,
        );
    }
}

impl<'player> PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>> {
    pub fn register_player(&mut self, player: &'player mut PokerBot) -> Result<(), String> {
        self.network.register_player(player)
    }

    pub async fn run(&mut self) -> Result<(), String> {
        self.network.run().await
    }
}

impl<'player> crum_net::PokerLobby
    for PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>>
{
    type Error = String;

    async fn run(&mut self) -> Result<(), Self::Error> {
        self.network.run().await
    }
}

#[allow(async_fn_in_trait)]
pub trait PokerArbitrumBotExt<'player> {
    async fn create_table(
        &'player mut self,
        lobby: &mut PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>>,
        table_name: &str,
        buy_in: u64,
        num_players: u8,
    ) -> Result<chain::TableCreatedOnChain, String>;

    async fn join_table_by_name(
        &'player mut self,
        lobby: &mut PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>>,
        table_name: &str,
    ) -> Result<chain::TableJoinedOnChain, String>;

    async fn join_table(
        &'player mut self,
        lobby: &mut PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>>,
        table_id: u64,
    ) -> Result<chain::TableJoinedOnChain, String>;
}

impl<'player> PokerArbitrumBotExt<'player> for PokerBot {
    async fn create_table(
        &'player mut self,
        lobby: &mut PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>>,
        table_name: &str,
        buy_in: u64,
        num_players: u8,
    ) -> Result<chain::TableCreatedOnChain, String> {
        let created = lobby.create_table(table_name, buy_in, num_players).await?;
        self.track_arbitrum_game(
            PokerClientState::new(
                created.seat_number,
                created.table_id,
                HAND_ID,
                new_table_with_players(num_players as usize).map_err(|err| err.to_string())?,
            ),
            lobby.lobby_id(),
            created.table_id,
            created.seat_number,
        );
        lobby.register_player(self)?;
        Ok(created)
    }

    async fn join_table_by_name(
        &'player mut self,
        lobby: &mut PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>>,
        table_name: &str,
    ) -> Result<chain::TableJoinedOnChain, String> {
        let table = lobby
            .find_table_by_name(table_name)
            .await?
            .ok_or_else(|| format!("table `{table_name}` was not found"))?;
        self.join_table(lobby, table.table_id).await
    }

    async fn join_table(
        &'player mut self,
        lobby: &mut PokerArbitrumLobby<PeerToPeerNetwork<'player, PokerPeerToPeerConnector>>,
        table_id: u64,
    ) -> Result<chain::TableJoinedOnChain, String> {
        let joined = lobby.join_table(table_id).await?;
        let detail = lobby
            .table_client
            .get_table_detail(lobby.details.lobby_id, table_id)
            .await?;
        let num_players = detail.info.flags.to::<usize>();
        self.track_arbitrum_game(
            PokerClientState::new(
                joined.seat_number,
                joined.table_id,
                HAND_ID,
                new_table_with_players(num_players).map_err(|err| err.to_string())?,
            ),
            lobby.lobby_id(),
            joined.table_id,
            joined.seat_number,
        );
        lobby.register_player(self)?;
        Ok(joined)
    }
}
