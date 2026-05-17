//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::PokerNetwork;

use crate::{chain, network::PokerPeerToPeerDetails};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokerArbitrumDetails {
    rpc_url: Option<String>,
    private_key: String,
    lobby_address: String,
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
            lobby_id,
        }
    }
}

pub struct PokerArbitrumLobby<N>
where
    N: PokerNetwork + PokerPeerToPeerDetails,
{
    network: N,
    details: PokerArbitrumDetails,
    _chain: chain::ChainConnection,
}

impl<N> PokerArbitrumLobby<N>
where
    N: PokerNetwork + PokerPeerToPeerDetails,
    N::Error: ToString,
{
    pub async fn use_network(network: N, details: PokerArbitrumDetails) -> Result<Self, String> {
        let chain = chain::connect(details.rpc_url.as_deref()).await?;
        tracing::info!(
            "connected to Nitro/Arbitrum RPC: url={}, chain_id={}",
            chain.rpc_url,
            chain.chain_id
        );

        Ok(Self {
            network,
            details,
            _chain: chain,
        })
    }

    pub async fn create_table(
        &mut self,
        table_name: &str,
        buy_in: u64,
    ) -> Result<chain::TableCreatedOnChain, String> {
        let created = chain::create_table(
            self.details.rpc_url.as_deref(),
            &self.details.private_key,
            &self.details.lobby_address,
            self.details.lobby_id,
            table_name,
            buy_in,
            self.network.annonce_public_key_hex(),
        )
        .await?;
        self.network
            .mark_table_ready()
            .map_err(|err| err.to_string())?;
        Ok(created)
    }

    pub async fn join_table(&mut self, table_id: u64) -> Result<chain::TableJoinedOnChain, String> {
        let joined = chain::join_table(
            self.details.rpc_url.as_deref(),
            &self.details.private_key,
            &self.details.lobby_address,
            self.details.lobby_id,
            table_id,
            self.network.annonce_public_key_hex(),
        )
        .await?;
        self.network
            .mark_table_ready()
            .map_err(|err| err.to_string())?;
        Ok(joined)
    }

    pub fn lobby_id(&self) -> u64 {
        self.details.lobby_id
    }

    pub fn into_network(self) -> N {
        self.network
    }
}
