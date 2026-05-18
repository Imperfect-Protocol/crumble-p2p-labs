//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use alloy_network::EthereumWallet;
use alloy_primitives::{Address, Bytes, U256, keccak256};
use alloy_provider::{DynProvider, Provider, ProviderBuilder};
use alloy_rpc_types_eth::{
    BlockNumberOrTag, Filter, Log, TransactionRequest, request::TransactionInput,
};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{SolCall, SolValue, sol};
use k256::ecdsa::SigningKey;

sol! {
    struct LobbyInfoView {
        uint256 lobby_id;
        uint256 lobby_game_type;
        uint256 lobby_flags;
        uint256 lobby_table_count;
        uint256 lobby_player_count;
        uint256 lobby_total_volume;
        string lobby_name;
    }

    struct TableInfoView {
        uint256 table_id;
        uint256 table_flags;
        uint256 table_buyin;
        uint256 table_player_count;
        uint256 table_total_buyin;
        string table_name;
    }

    struct TablePlayerInfoView {
        address player_address;
        uint256 player_chips;
        bytes player_annonce_public_key;
    }

    struct TableDetailView {
        TableInfoView info;
        TablePlayerInfoView[] players;
    }

    interface IPrivatePokerLobby {
        function createTable(uint256 lobby_id, uint256 table_id, string name, uint256 buy_in, uint256 num_players, bytes annonce_public_key) external;
        function joinTable(uint256 lobby_id, uint256 table_id, bytes annonce_public_key) external;
        function getLobbyById(uint256 lobby_id) external view returns (bytes);
        function getTablesRange(uint256 lobby_id, uint256 offset, uint256 count) external view returns (bytes[]);
        function getTableDetail(uint256 lobby_id, uint256 table_id) external view returns (bytes);
        event PlayerJoined(address player_address, uint256 lobby_id, uint256 table_id, string player_name, uint256 player_chips);
    }

    interface IPrivatePokerSignal {
        function sendSignal(uint256 lobby_id, uint256 table_id, address[] recipients, bytes[] encrypted_data) external;
        event HandshakeSignal(address sender, uint256 lobby_id, uint256 table_id, address[] recipients, bytes[] encrypted_data);
    }

    interface IPrivatePokerHand {
        function startHand(uint256 lobby_id, uint256 table_id) external;
        event HandStarted(uint256 lobby_id, uint256 table_id, uint256 seat_number, uint256 remain_count);
    }
}

pub const DEFAULT_RPC_URL: &str = "http://localhost:8547";

#[derive(Clone)]
pub struct ChainConnector {
    pub rpc_url: String,
    pub chain_id: u64,
    pub wallet_address: Address,
    tx_provider: DynProvider,
    event_provider: DynProvider,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCreatedOnChain {
    pub lobby_id: u64,
    pub table_id: u64,
    pub seat_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableJoinedOnChain {
    pub lobby_id: u64,
    pub table_id: u64,
    pub seat_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbyInfoOnChain {
    pub lobby_id: u64,
    pub table_count: u64,
    pub player_count: u64,
    pub total_volume: U256,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInfoOnChain {
    pub table_id: u64,
    pub flags: U256,
    pub buy_in: U256,
    pub player_count: u64,
    pub total_buy_in: U256,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TablePlayerOnChain {
    pub address: Address,
    pub chips: U256,
    pub annonce_public_key_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDetailOnChain {
    pub info: TableInfoOnChain,
    pub players: Vec<TablePlayerOnChain>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeSignalOnChain {
    pub sender: Address,
    pub recipient_index: usize,
    pub encrypted_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandStartedOnChain {
    pub seat_number: U256,
    pub remain_count: U256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerJoinedOnChain {
    pub player_address: Address,
}

#[derive(Clone)]
pub struct PrivatePokerLobbyClient {
    connector: ChainConnector,
    address: String,
}

#[derive(Clone)]
pub struct PrivatePokerTableClient {
    connector: ChainConnector,
    address: String,
    seen_logs: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone)]
pub struct PrivatePokerSpectateClient {
    connector: ChainConnector,
    address: String,
}

#[derive(Clone)]
pub struct PrivatePokerSignalClient {
    connector: ChainConnector,
    address: String,
    seen_logs: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone)]
pub struct PrivatePokerHandClient {
    connector: ChainConnector,
    address: String,
    seen_logs: Arc<Mutex<HashSet<String>>>,
}

impl PrivatePokerLobbyClient {
    pub fn new(connector: ChainConnector, address: impl Into<String>) -> Self {
        Self {
            connector,
            address: address.into(),
        }
    }
}

impl PrivatePokerTableClient {
    pub fn new(connector: ChainConnector, address: impl Into<String>) -> Self {
        Self {
            connector,
            address: address.into(),
            seen_logs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn poll_player_joined(
        &self,
        lobby_id: u64,
        table_id: u64,
    ) -> Result<Vec<PlayerJoinedOnChain>, String> {
        let event_provider = self.connector.new_event_provider()?;
        let table_address = Address::from_str(&self.address)
            .map_err(|err| format!("invalid table address `{}`: {err}", self.address))?;
        let filter = Filter::new()
            .address(table_address)
            .event("PlayerJoined(address,uint256,uint256,string,uint256)")
            .from_block(0u64)
            .to_block(BlockNumberOrTag::Latest);
        let logs = event_provider
            .get_logs(&filter)
            .await
            .map_err(|err| format!("failed to poll PlayerJoined logs: {err}"))?;

        decode_player_joined(logs, lobby_id, table_id, &self.seen_logs)
    }
}

impl PrivatePokerSpectateClient {
    pub fn new(connector: ChainConnector, address: impl Into<String>) -> Self {
        Self {
            connector,
            address: address.into(),
        }
    }
}

impl PrivatePokerSignalClient {
    pub fn new(connector: ChainConnector, address: impl Into<String>) -> Self {
        Self {
            connector,
            address: address.into(),
            seen_logs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn poll_handshake_signals(
        &self,
        lobby_id: u64,
        table_id: u64,
    ) -> Result<Vec<HandshakeSignalOnChain>, String> {
        let event_provider = self.connector.new_event_provider()?;
        let signal_address = Address::from_str(&self.address)
            .map_err(|err| format!("invalid signal address `{}`: {err}", self.address))?;
        let filter = Filter::new()
            .address(signal_address)
            .event("HandshakeSignal(address,uint256,uint256,address[],bytes[])")
            .from_block(0u64)
            .to_block(BlockNumberOrTag::Latest);
        let logs = event_provider
            .get_logs(&filter)
            .await
            .map_err(|err| format!("failed to poll HandshakeSignal logs: {err}"))?;

        decode_matching_signals(
            logs,
            lobby_id,
            table_id,
            self.connector.wallet_address,
            &self.seen_logs,
        )
    }
}

impl PrivatePokerHandClient {
    pub fn new(connector: ChainConnector, address: impl Into<String>) -> Self {
        Self {
            connector,
            address: address.into(),
            seen_logs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn poll_hand_started(
        &self,
        lobby_id: u64,
        table_id: u64,
    ) -> Result<Vec<HandStartedOnChain>, String> {
        let event_provider = self.connector.new_event_provider()?;
        let hand_address = Address::from_str(&self.address)
            .map_err(|err| format!("invalid hand address `{}`: {err}", self.address))?;
        let filter = Filter::new()
            .address(hand_address)
            .event("HandStarted(uint256,uint256,uint256,uint256)")
            .from_block(0u64)
            .to_block(BlockNumberOrTag::Latest);
        let logs = event_provider
            .get_logs(&filter)
            .await
            .map_err(|err| format!("failed to poll HandStarted logs: {err}"))?;

        let mut events = Vec::new();
        while let Some(event) =
            decode_hand_started(logs.clone(), lobby_id, table_id, &self.seen_logs)?
        {
            events.push(event);
        }
        Ok(events)
    }
}

#[allow(async_fn_in_trait)]
pub trait PrivatePokerLobbyContract {
    async fn create_table(
        &self,
        lobby_id: u64,
        table_name: &str,
        buy_in: u64,
        num_players: u8,
        annonce_public_key_hex: &str,
    ) -> Result<TableCreatedOnChain, String>;
}

#[allow(async_fn_in_trait)]
pub trait PrivatePokerTableContract {
    async fn join_table(
        &self,
        lobby_id: u64,
        table_id: u64,
        annonce_public_key_hex: &str,
    ) -> Result<TableJoinedOnChain, String>;

    async fn get_table_detail(
        &self,
        lobby_id: u64,
        table_id: u64,
    ) -> Result<TableDetailOnChain, String>;
}

#[allow(async_fn_in_trait)]
pub trait PrivatePokerSpectateContract {
    async fn get_lobby_by_id(&self, lobby_id: u64) -> Result<LobbyInfoOnChain, String>;

    async fn get_tables_range(
        &self,
        lobby_id: u64,
        offset: u64,
        count: u64,
    ) -> Result<Vec<TableInfoOnChain>, String>;

    async fn find_table_by_name(
        &self,
        lobby_id: u64,
        table_name: &str,
    ) -> Result<Option<TableInfoOnChain>, String>;
}

#[allow(async_fn_in_trait)]
pub trait PrivatePokerSignalContract {
    async fn send_signal(
        &self,
        lobby_id: u64,
        table_id: u64,
        recipient: Address,
        encrypted_data: Vec<u8>,
    ) -> Result<(), String>;

    async fn wait_handshake_signal(
        &self,
        lobby_id: u64,
        table_id: u64,
        peer: Address,
    ) -> Result<HandshakeSignalOnChain, String>;
}

#[allow(async_fn_in_trait)]
pub trait PrivatePokerHandContract {
    async fn start_hand(&self, lobby_id: u64, table_id: u64) -> Result<(), String>;

    async fn wait_hand_started(
        &self,
        lobby_id: u64,
        table_id: u64,
    ) -> Result<HandStartedOnChain, String>;
}

impl ChainConnector {
    pub async fn connect(rpc_url: Option<&str>, private_key: &str) -> Result<Self, String> {
        let rpc_url = rpc_url.unwrap_or(DEFAULT_RPC_URL).to_string();
        let parsed_url = rpc_url
            .parse()
            .map_err(|err| format!("invalid rpc url `{rpc_url}`: {err}"))?;
        let event_url = rpc_url
            .parse()
            .map_err(|err| format!("invalid rpc url `{rpc_url}`: {err}"))?;
        let wallet_address = address_from_private_key(private_key)?;
        let signer = PrivateKeySigner::from_str(private_key)
            .map_err(|err| format!("invalid signing private key: {err}"))?;
        let wallet = EthereumWallet::from(signer);
        let tx_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(parsed_url)
            .erased();
        let event_provider = ProviderBuilder::new().on_http(event_url).erased();
        let chain_id = tx_provider
            .get_chain_id()
            .await
            .map_err(|err| format!("failed to connect to `{rpc_url}`: {err}"))?;

        Ok(Self {
            rpc_url,
            chain_id,
            wallet_address,
            tx_provider,
            event_provider,
        })
    }

    async fn send_transaction(&self, to: &str, calldata: Vec<u8>) -> Result<(), String> {
        let to = Address::from_str(to).map_err(|err| format!("invalid address `{to}`: {err}"))?;
        let tx = TransactionRequest::default()
            .from(self.wallet_address)
            .to(to)
            .input(TransactionInput::new(Bytes::from(calldata)));

        let receipt = self
            .tx_provider
            .send_transaction(tx)
            .await
            .map_err(|err| {
                format!(
                    "failed to send signed transaction from {} via `{}`: {err}",
                    self.wallet_address, self.rpc_url
                )
            })?
            .get_receipt()
            .await
            .map_err(|err| format!("failed waiting for transaction receipt: {err}"))?;

        if !receipt.status() {
            return Err(format!(
                "transaction reverted: {:?}",
                receipt.transaction_hash
            ));
        }

        Ok(())
    }

    async fn call_view(&self, to: &str, calldata: Vec<u8>) -> Result<Bytes, String> {
        let to = Address::from_str(to).map_err(|err| format!("invalid address `{to}`: {err}"))?;
        let tx = TransactionRequest::default()
            .to(to)
            .input(TransactionInput::new(Bytes::from(calldata)));

        self.event_provider.call(&tx).await.map_err(|err| {
            format!(
                "failed to call contract at {to} via `{}`: {err}",
                self.rpc_url
            )
        })
    }

    fn new_event_provider(&self) -> Result<DynProvider, String> {
        let url = self
            .rpc_url
            .parse()
            .map_err(|err| format!("invalid rpc url `{}`: {err}", self.rpc_url))?;
        Ok(ProviderBuilder::new().on_http(url).erased())
    }
}

impl PrivatePokerLobbyContract for PrivatePokerLobbyClient {
    async fn create_table(
        &self,
        lobby_id: u64,
        table_name: &str,
        buy_in: u64,
        num_players: u8,
        annonce_public_key_hex: &str,
    ) -> Result<TableCreatedOnChain, String> {
        let table_id = generate_table_id()?;
        let call = IPrivatePokerLobby::createTableCall {
            lobby_id: U256::from(lobby_id),
            table_id: U256::from(table_id),
            name: table_name.to_string(),
            buy_in: U256::from(buy_in),
            num_players: U256::from(num_players),
            annonce_public_key: decode_hex_bytes(annonce_public_key_hex)?.into(),
        };

        self.connector
            .send_transaction(&self.address, call.abi_encode())
            .await?;
        Ok(TableCreatedOnChain {
            lobby_id,
            table_id,
            seat_number: 0,
        })
    }
}

impl PrivatePokerTableContract for PrivatePokerTableClient {
    async fn join_table(
        &self,
        lobby_id: u64,
        table_id: u64,
        annonce_public_key_hex: &str,
    ) -> Result<TableJoinedOnChain, String> {
        let call = IPrivatePokerLobby::joinTableCall {
            lobby_id: U256::from(lobby_id),
            table_id: U256::from(table_id),
            annonce_public_key: decode_hex_bytes(annonce_public_key_hex)?.into(),
        };

        self.connector
            .send_transaction(&self.address, call.abi_encode())
            .await?;
        let detail = self.get_table_detail(lobby_id, table_id).await?;
        let seat_number = detail
            .players
            .iter()
            .position(|player| player.address == self.connector.wallet_address)
            .ok_or_else(|| "joined table does not contain local wallet".to_string())?;

        Ok(TableJoinedOnChain {
            lobby_id,
            table_id,
            seat_number,
        })
    }

    async fn get_table_detail(
        &self,
        lobby_id: u64,
        table_id: u64,
    ) -> Result<TableDetailOnChain, String> {
        let call = IPrivatePokerLobby::getTableDetailCall {
            lobby_id: U256::from(lobby_id),
            table_id: U256::from(table_id),
        };
        let raw = self
            .connector
            .call_view(&self.address, call.abi_encode())
            .await?;
        let returned = IPrivatePokerLobby::getTableDetailCall::abi_decode_returns(&raw, true)
            .map_err(|err| format!("failed to decode get_table_detail return: {err}"))?;
        let detail = TableDetailView::abi_decode(returned._0.as_ref(), true)
            .map_err(|err| format!("failed to decode table detail bytes: {err}"))?;

        Ok(TableDetailOnChain {
            info: table_info_from_view(detail.info)?,
            players: detail
                .players
                .into_iter()
                .map(|player| TablePlayerOnChain {
                    address: player.player_address,
                    chips: player.player_chips,
                    annonce_public_key_hex: hex::encode(player.player_annonce_public_key),
                })
                .collect(),
        })
    }
}

impl PrivatePokerSpectateContract for PrivatePokerSpectateClient {
    async fn get_lobby_by_id(&self, lobby_id: u64) -> Result<LobbyInfoOnChain, String> {
        let call = IPrivatePokerLobby::getLobbyByIdCall {
            lobby_id: U256::from(lobby_id),
        };
        let raw = self
            .connector
            .call_view(&self.address, call.abi_encode())
            .await?;
        let returned = IPrivatePokerLobby::getLobbyByIdCall::abi_decode_returns(&raw, true)
            .map_err(|err| format!("failed to decode get_lobby_by_id return: {err}"))?;
        let info = LobbyInfoView::abi_decode(returned._0.as_ref(), true)
            .map_err(|err| format!("failed to decode lobby info bytes: {err}"))?;

        Ok(LobbyInfoOnChain {
            lobby_id: u256_to_u64(info.lobby_id, "lobby_id")?,
            table_count: u256_to_u64(info.lobby_table_count, "lobby_table_count")?,
            player_count: u256_to_u64(info.lobby_player_count, "lobby_player_count")?,
            total_volume: info.lobby_total_volume,
            name: info.lobby_name,
        })
    }

    async fn get_tables_range(
        &self,
        lobby_id: u64,
        offset: u64,
        count: u64,
    ) -> Result<Vec<TableInfoOnChain>, String> {
        let call = IPrivatePokerLobby::getTablesRangeCall {
            lobby_id: U256::from(lobby_id),
            offset: U256::from(offset),
            count: U256::from(count),
        };
        let raw = self
            .connector
            .call_view(&self.address, call.abi_encode())
            .await?;
        let returned = IPrivatePokerLobby::getTablesRangeCall::abi_decode_returns(&raw, true)
            .map_err(|err| format!("failed to decode get_tables_range return: {err}"))?;

        returned
            ._0
            .into_iter()
            .map(|encoded| {
                let info = TableInfoView::abi_decode(encoded.as_ref(), true)
                    .map_err(|err| format!("failed to decode table info bytes: {err}"))?;
                Ok(TableInfoOnChain {
                    table_id: u256_to_u64(info.table_id, "table_id")?,
                    flags: info.table_flags,
                    buy_in: info.table_buyin,
                    player_count: u256_to_u64(info.table_player_count, "table_player_count")?,
                    total_buy_in: info.table_total_buyin,
                    name: info.table_name,
                })
            })
            .collect()
    }

    async fn find_table_by_name(
        &self,
        lobby_id: u64,
        table_name: &str,
    ) -> Result<Option<TableInfoOnChain>, String> {
        let lobby = self.get_lobby_by_id(lobby_id).await?;
        let mut offset = 0u64;
        while offset < lobby.table_count {
            let count = (lobby.table_count - offset).min(50);
            let tables = self.get_tables_range(lobby_id, offset, count).await?;
            if let Some(table) = tables.into_iter().find(|table| table.name == table_name) {
                return Ok(Some(table));
            }
            offset += count;
        }
        Ok(None)
    }
}

impl PrivatePokerSignalContract for PrivatePokerSignalClient {
    async fn send_signal(
        &self,
        lobby_id: u64,
        table_id: u64,
        recipient: Address,
        encrypted_data: Vec<u8>,
    ) -> Result<(), String> {
        let call = IPrivatePokerSignal::sendSignalCall {
            lobby_id: U256::from(lobby_id),
            table_id: U256::from(table_id),
            recipients: vec![recipient],
            encrypted_data: vec![Bytes::from(encrypted_data)],
        };
        self.connector
            .send_transaction(&self.address, call.abi_encode())
            .await
    }

    async fn wait_handshake_signal(
        &self,
        lobby_id: u64,
        table_id: u64,
        peer: Address,
    ) -> Result<HandshakeSignalOnChain, String> {
        let event_provider = self.connector.new_event_provider()?;
        let signal_address = Address::from_str(&self.address)
            .map_err(|err| format!("invalid signal address `{}`: {err}", self.address))?;
        loop {
            let latest = event_provider
                .get_block_number()
                .await
                .map_err(|err| format!("failed to get block number for signal watch: {err}"))?;
            let filter = Filter::new()
                .address(signal_address)
                .event("HandshakeSignal(address,uint256,uint256,address[],bytes[])")
                .from_block(0u64)
                .to_block(BlockNumberOrTag::Latest);
            let logs = event_provider
                .get_logs(&filter)
                .await
                .map_err(|err| format!("failed to poll HandshakeSignal logs: {err}"))?;

            if let Some(signal) = decode_matching_signal(
                logs,
                lobby_id,
                table_id,
                peer,
                self.connector.wallet_address,
                &self.seen_logs,
            )? {
                return Ok(signal);
            }

            tracing::debug!(
                "waiting for HandshakeSignal from {} to {} at block {}",
                peer,
                self.connector.wallet_address,
                latest
            );
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}

impl PrivatePokerHandContract for PrivatePokerHandClient {
    async fn start_hand(&self, lobby_id: u64, table_id: u64) -> Result<(), String> {
        let call = IPrivatePokerHand::startHandCall {
            lobby_id: U256::from(lobby_id),
            table_id: U256::from(table_id),
        };
        match self
            .connector
            .send_transaction(&self.address, call.abi_encode())
            .await
        {
            Ok(()) => Ok(()),
            Err(err) if err.contains("0x414c52454144595f5245414459") => Ok(()),
            Err(err) => Err(err),
        }
    }

    async fn wait_hand_started(
        &self,
        lobby_id: u64,
        table_id: u64,
    ) -> Result<HandStartedOnChain, String> {
        let event_provider = self.connector.new_event_provider()?;
        let hand_address = Address::from_str(&self.address)
            .map_err(|err| format!("invalid hand address `{}`: {err}", self.address))?;
        loop {
            let latest = event_provider.get_block_number().await.map_err(|err| {
                format!("failed to get block number for HandStarted watch: {err}")
            })?;
            let filter = Filter::new()
                .address(hand_address)
                .event("HandStarted(uint256,uint256,uint256,uint256)")
                .from_block(0u64)
                .to_block(BlockNumberOrTag::Latest);
            let logs = event_provider
                .get_logs(&filter)
                .await
                .map_err(|err| format!("failed to poll HandStarted logs: {err}"))?;

            if let Some(hand_started) =
                decode_hand_started(logs, lobby_id, table_id, &self.seen_logs)?
            {
                return Ok(hand_started);
            }

            tracing::debug!(
                "waiting for HandStarted for lobby {} table {} at block {}",
                lobby_id,
                table_id,
                latest
            );
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}

fn decode_matching_signal(
    logs: Vec<Log>,
    lobby_id: u64,
    table_id: u64,
    peer: Address,
    wallet: Address,
    seen_logs: &Mutex<HashSet<String>>,
) -> Result<Option<HandshakeSignalOnChain>, String> {
    for log in logs {
        let log_id = format!("{:?}", log);
        if seen_logs
            .lock()
            .map_err(|_| "HandshakeSignal log cursor is poisoned".to_string())?
            .contains(&log_id)
        {
            continue;
        }
        let Ok(decoded) = log.log_decode::<IPrivatePokerSignal::HandshakeSignal>() else {
            continue;
        };
        let signal = decoded.inner.data;
        if signal.sender != peer {
            continue;
        }
        if signal.lobby_id != U256::from(lobby_id) || signal.table_id != U256::from(table_id) {
            continue;
        }
        for (recipient_index, (recipient, encrypted_data)) in signal
            .recipients
            .iter()
            .zip(signal.encrypted_data)
            .enumerate()
        {
            if *recipient == wallet {
                seen_logs
                    .lock()
                    .map_err(|_| "HandshakeSignal log cursor is poisoned".to_string())?
                    .insert(log_id);
                return Ok(Some(HandshakeSignalOnChain {
                    sender: signal.sender,
                    recipient_index,
                    encrypted_data: encrypted_data.to_vec(),
                }));
            }
        }
    }
    Ok(None)
}

fn decode_matching_signals(
    logs: Vec<Log>,
    lobby_id: u64,
    table_id: u64,
    wallet: Address,
    seen_logs: &Mutex<HashSet<String>>,
) -> Result<Vec<HandshakeSignalOnChain>, String> {
    let mut signals = Vec::new();
    for log in logs {
        let log_id = format!("{:?}", log);
        if seen_logs
            .lock()
            .map_err(|_| "HandshakeSignal log cursor is poisoned".to_string())?
            .contains(&log_id)
        {
            continue;
        }
        let Ok(decoded) = log.log_decode::<IPrivatePokerSignal::HandshakeSignal>() else {
            continue;
        };
        let signal = decoded.inner.data;
        if signal.lobby_id != U256::from(lobby_id) || signal.table_id != U256::from(table_id) {
            continue;
        }
        for (recipient_index, (recipient, encrypted_data)) in signal
            .recipients
            .iter()
            .zip(signal.encrypted_data)
            .enumerate()
        {
            if *recipient == wallet {
                seen_logs
                    .lock()
                    .map_err(|_| "HandshakeSignal log cursor is poisoned".to_string())?
                    .insert(log_id.clone());
                signals.push(HandshakeSignalOnChain {
                    sender: signal.sender,
                    recipient_index,
                    encrypted_data: encrypted_data.to_vec(),
                });
                break;
            }
        }
    }
    Ok(signals)
}

fn decode_hand_started(
    logs: Vec<Log>,
    lobby_id: u64,
    table_id: u64,
    seen_logs: &Mutex<HashSet<String>>,
) -> Result<Option<HandStartedOnChain>, String> {
    for log in logs {
        let log_id = format!("{:?}", log);
        if seen_logs
            .lock()
            .map_err(|_| "HandStarted log cursor is poisoned".to_string())?
            .contains(&log_id)
        {
            continue;
        }
        let Ok(decoded) = log.log_decode::<IPrivatePokerHand::HandStarted>() else {
            continue;
        };
        let event = decoded.inner.data;
        if event.lobby_id != U256::from(lobby_id) || event.table_id != U256::from(table_id) {
            continue;
        }
        seen_logs
            .lock()
            .map_err(|_| "HandStarted log cursor is poisoned".to_string())?
            .insert(log_id);
        return Ok(Some(HandStartedOnChain {
            seat_number: event.seat_number,
            remain_count: event.remain_count,
        }));
    }
    Ok(None)
}

fn decode_player_joined(
    logs: Vec<Log>,
    lobby_id: u64,
    table_id: u64,
    seen_logs: &Mutex<HashSet<String>>,
) -> Result<Vec<PlayerJoinedOnChain>, String> {
    let mut events = Vec::new();
    for log in logs {
        let log_id = format!("{:?}", log);
        if seen_logs
            .lock()
            .map_err(|_| "PlayerJoined log cursor is poisoned".to_string())?
            .contains(&log_id)
        {
            continue;
        }
        let Ok(decoded) = log.log_decode::<IPrivatePokerLobby::PlayerJoined>() else {
            continue;
        };
        let event = decoded.inner.data;
        if event.lobby_id != U256::from(lobby_id) || event.table_id != U256::from(table_id) {
            continue;
        }
        seen_logs
            .lock()
            .map_err(|_| "PlayerJoined log cursor is poisoned".to_string())?
            .insert(log_id);
        events.push(PlayerJoinedOnChain {
            player_address: event.player_address,
        });
    }
    Ok(events)
}

fn table_info_from_view(info: TableInfoView) -> Result<TableInfoOnChain, String> {
    Ok(TableInfoOnChain {
        table_id: u256_to_u64(info.table_id, "table_id")?,
        flags: info.table_flags,
        buy_in: info.table_buyin,
        player_count: u256_to_u64(info.table_player_count, "table_player_count")?,
        total_buy_in: info.table_total_buyin,
        name: info.table_name,
    })
}

pub fn address_from_private_key(private_key: &str) -> Result<Address, String> {
    let key_bytes = decode_fixed_32(private_key)?;
    let signing_key = SigningKey::from_slice(&key_bytes)
        .map_err(|err| format!("invalid secp256k1 private key: {err}"))?;
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let public_key = encoded.as_bytes();
    if public_key.len() != 65 {
        return Err("unexpected secp256k1 public key length".to_string());
    }

    let hash = keccak256(&public_key[1..]);
    Ok(Address::from_slice(&hash[12..]))
}

fn decode_hex_bytes(value: &str) -> Result<Vec<u8>, String> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    hex::decode(value).map_err(|err| format!("invalid hex bytes: {err}"))
}

fn decode_fixed_32(value: &str) -> Result<[u8; 32], String> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(value, &mut bytes)
        .map_err(|err| format!("expected 32-byte private key hex: {err}"))?;
    Ok(bytes)
}

fn u256_to_u64(value: U256, field: &str) -> Result<u64, String> {
    value
        .try_into()
        .map_err(|_| format!("field `{field}` does not fit into u64: {value}"))
}

fn generate_table_id() -> Result<u64, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system time before Unix epoch: {err}"))?;
    let micros = now.as_micros();
    let id = micros % 9_000_000_000_000_000 + 1_000_000;
    u64::try_from(id).map_err(|err| format!("table id overflow: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_anvil_account_address() {
        let address = address_from_private_key(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .expect("address");

        assert_eq!(
            address,
            Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()
        );
    }
}
