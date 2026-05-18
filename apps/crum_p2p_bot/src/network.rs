//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{marker::PhantomData, sync::Arc, time::Duration};

use bytes::Bytes as DataBytes;
use crum_bls::signal_crypto::{decrypt_signal_from_peer, encrypt_signal_for_peer};
use crum_net::{
    NetworkConnector, PlayerEvent, PokerActionEnvelopeWire, PokerPlayerRegistrar, TopologyPolicy,
    current_player, final_lines, is_finished,
};
use tokio::sync::mpsc;
use webrtc::{
    api::{APIBuilder, media_engine::MediaEngine},
    data_channel::{
        RTCDataChannel, data_channel_message::DataChannelMessage,
        data_channel_state::RTCDataChannelState,
    },
    ice_transport::ice_server::RTCIceServer,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        sdp::session_description::RTCSessionDescription,
    },
};

use crate::bot::PokerBot;
use crate::chain::{
    HandshakeSignalOnChain, PrivatePokerHandClient, PrivatePokerHandContract,
    PrivatePokerSignalClient, PrivatePokerSignalContract, PrivatePokerTableClient,
    PrivatePokerTableContract,
};

pub struct PokerPeerToPeerConnector {
    pub stun_server: String,
    pub player_details: PokerPeerToPeerPlayerDetails,
    inbound_peer_connection: Option<RTCPeerConnection>,
    inbound_data_channel: Option<Arc<RTCDataChannel>>,
    outbound_peer_connection: Option<RTCPeerConnection>,
    outbound_data_channel: Option<Arc<RTCDataChannel>>,
    outbound_data_channel_rx: Option<mpsc::Receiver<Arc<RTCDataChannel>>>,
    message_tx: mpsc::Sender<Vec<u8>>,
    message_rx: Option<mpsc::Receiver<Vec<u8>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokerPeerToPeerPlayerDetails {
    pub annonce_public_key_hex: String,
    pub annonce_private_key_hex: String,
}

impl PokerPeerToPeerConnector {
    pub fn new(
        stun_server: impl Into<String>,
        player_details: PokerPeerToPeerPlayerDetails,
    ) -> Self {
        let (message_tx, message_rx) = mpsc::channel::<Vec<u8>>(256);
        Self {
            stun_server: stun_server.into(),
            player_details,
            inbound_peer_connection: None,
            inbound_data_channel: None,
            outbound_peer_connection: None,
            outbound_data_channel: None,
            outbound_data_channel_rx: None,
            message_tx,
            message_rx: Some(message_rx),
        }
    }

    pub async fn get_connection_info(&mut self) -> Result<Vec<u8>, String> {
        let api = webrtc_api();
        let peer_connection = new_peer_connection(&api, &self.stun_server).await?;
        let data_channel = peer_connection
            .create_data_channel("private-poker", None)
            .await
            .map_err(|err| err.to_string())?;
        attach_message_handler(&data_channel, self.message_tx.clone());

        let mut gathering_complete = peer_connection.gathering_complete_promise().await;
        let offer = peer_connection
            .create_offer(None)
            .await
            .map_err(|err| err.to_string())?;
        peer_connection
            .set_local_description(offer)
            .await
            .map_err(|err| err.to_string())?;
        let _ = gathering_complete.recv().await;

        let session = peer_connection
            .local_description()
            .await
            .ok_or_else(|| "WebRTC local description missing".to_string())?;
        self.inbound_peer_connection = Some(peer_connection);
        self.inbound_data_channel = Some(data_channel);
        nson::encode::to_bytes(&SignallingMessage::Offer(WebRtcConnectionInfo { session }))
            .map_err(|err| format!("failed to encode WebRTC connection info: {err}"))
    }

    pub async fn connect_to_peer(&mut self, blob: Vec<u8>) -> Result<Vec<u8>, String> {
        let connection_info: WebRtcConnectionInfo = nson::decode::from_bytes(&blob)
            .map_err(|err| format!("failed to decode WebRTC connection info: {err}"))?;
        self.answer_offer(connection_info).await
    }

    async fn answer_offer(
        &mut self,
        connection_info: WebRtcConnectionInfo,
    ) -> Result<Vec<u8>, String> {
        let api = webrtc_api();
        let peer_connection = new_peer_connection(&api, &self.stun_server).await?;
        let (data_channel_tx, data_channel_rx) = mpsc::channel::<Arc<RTCDataChannel>>(1);
        let message_tx = self.message_tx.clone();
        peer_connection.on_data_channel(Box::new(move |data_channel: Arc<RTCDataChannel>| {
            let message_tx = message_tx.clone();
            let data_channel_tx = data_channel_tx.clone();
            Box::pin(async move {
                attach_message_handler(&data_channel, message_tx);
                let _ = data_channel_tx.send(data_channel).await;
            })
        }));
        peer_connection
            .set_remote_description(connection_info.session)
            .await
            .map_err(|err| err.to_string())?;
        let mut gathering_complete = peer_connection.gathering_complete_promise().await;
        let answer = peer_connection
            .create_answer(None)
            .await
            .map_err(|err| err.to_string())?;
        peer_connection
            .set_local_description(answer)
            .await
            .map_err(|err| err.to_string())?;
        let _ = gathering_complete.recv().await;
        let session = peer_connection
            .local_description()
            .await
            .ok_or_else(|| "WebRTC answer local description missing".to_string())?;
        self.outbound_peer_connection = Some(peer_connection);
        self.outbound_data_channel_rx = Some(data_channel_rx);
        nson::encode::to_bytes(&SignallingMessage::Answer(WebRtcConnectionInfo { session }))
            .map_err(|err| format!("failed to encode WebRTC answer: {err}"))
    }

    async fn accept_answer(&mut self, connection_info: WebRtcConnectionInfo) -> Result<(), String> {
        let peer_connection = self
            .inbound_peer_connection
            .as_ref()
            .ok_or_else(|| "WebRTC inbound offer connection is not open".to_string())?;
        peer_connection
            .set_remote_description(connection_info.session)
            .await
            .map_err(|err| err.to_string())?;
        let data_channel = self
            .inbound_data_channel
            .as_ref()
            .ok_or_else(|| "WebRTC inbound data channel is not open".to_string())?;
        wait_data_channel_open(data_channel).await
    }

    async fn connect_to_peer_from_signal(
        &mut self,
        signal: &HandshakeSignalOnChain,
    ) -> Result<SignallingOutcome, String> {
        let payload_bytes = decrypt_signal_from_peer(
            &self.player_details.annonce_private_key_hex,
            &signal.encrypted_data,
        )?;
        tracing::info!(
            "HandshakeSignal from {} matched local recipient index {}",
            signal.sender,
            signal.recipient_index
        );
        match nson::decode::from_bytes(&payload_bytes)
            .map_err(|err| format!("failed to decode signalling message: {err}"))?
        {
            SignallingMessage::Offer(connection_info) => self
                .answer_offer(connection_info)
                .await
                .map(SignallingOutcome::SendAnswer),
            SignallingMessage::Answer(connection_info) => {
                self.accept_answer(connection_info).await?;
                Ok(SignallingOutcome::ConnectionEstablished)
            }
        }
    }

    fn take_messages(&mut self) -> Result<mpsc::Receiver<Vec<u8>>, String> {
        self.message_rx
            .take()
            .ok_or_else(|| "p2p connector message receiver was already taken".to_string())
    }

    fn refresh_outbound_data_channel(&mut self) {
        if self.outbound_data_channel.is_some() {
            return;
        }
        if let Some(rx) = &mut self.outbound_data_channel_rx {
            if let Ok(data_channel) = rx.try_recv() {
                self.outbound_data_channel = Some(data_channel);
            }
        }
    }

    fn can_send(&mut self) -> bool {
        self.refresh_outbound_data_channel();
        self.outbound_data_channel.is_some()
    }

    async fn send(&mut self, payload: Vec<u8>) -> Result<(), String> {
        self.refresh_outbound_data_channel();
        let data_channel = self
            .outbound_data_channel
            .as_ref()
            .ok_or_else(|| "WebRTC data channel is not connected".to_string())?;
        data_channel
            .send(&DataBytes::from(payload))
            .await
            .map(|_| ())
            .map_err(|err| err.to_string())
    }

    async fn close(&self) -> Result<(), String> {
        if let Some(peer_connection) = &self.inbound_peer_connection {
            peer_connection
                .close()
                .await
                .map_err(|err| err.to_string())?;
        }
        if let Some(peer_connection) = &self.outbound_peer_connection {
            peer_connection
                .close()
                .await
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

impl NetworkConnector for PokerPeerToPeerConnector {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WebRtcConnectionInfo {
    session: RTCSessionDescription,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum SignallingMessage {
    Offer(WebRtcConnectionInfo),
    Answer(WebRtcConnectionInfo),
}

enum SignallingOutcome {
    SendAnswer(Vec<u8>),
    ConnectionEstablished,
}

fn webrtc_api() -> webrtc::api::API {
    APIBuilder::new()
        .with_media_engine(MediaEngine::default())
        .build()
}

async fn new_peer_connection(
    api: &webrtc::api::API,
    stun_server: &str,
) -> Result<RTCPeerConnection, String> {
    api.new_peer_connection(RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec![stun_server.to_string()],
            ..Default::default()
        }],
        ..Default::default()
    })
    .await
    .map_err(|err| err.to_string())
}

#[allow(async_fn_in_trait)]
pub trait PokerPeerToPeerDetails {
    fn annonce_public_key_hex(&self) -> &str;
    fn install_chain(
        &mut self,
        table_client: PrivatePokerTableClient,
        hand_client: PrivatePokerHandClient,
        signal_client: PrivatePokerSignalClient,
        _wallet_address: alloy_primitives::Address,
        lobby_id: u64,
        table_id: u64,
    );
    async fn get_connection_info(&mut self) -> Result<Vec<u8>, String>;
    async fn connect_to_peer(&mut self, blob: Vec<u8>) -> Result<Vec<u8>, String>;
    fn prepare_table_creator(&mut self);
    fn prepare_table_joiner(&mut self);
}

pub struct PeerToPeerNetwork<'player, Connector = PokerPeerToPeerConnector>
where
    Connector: NetworkConnector,
{
    runtime: PokerPeerToPeerRuntime<'player>,
    connector_type: PhantomData<Connector>,
}

pub struct PokerPeerToPeerRuntime<'player> {
    connector: PokerPeerToPeerConnector,
    topology: TopologyPolicy,
    endpoint: Option<PokerPeerToPeerEndpoint>,
    player: Option<&'player mut PokerBot>,
    chain: Option<PokerPeerToPeerChain>,
}

#[derive(Clone)]
struct PokerPeerToPeerChain {
    hand_client: PrivatePokerHandClient,
    signal_client: PrivatePokerSignalClient,
    table_client: PrivatePokerTableClient,
    wallet_address: alloy_primitives::Address,
    lobby_id: u64,
    table_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PokerPeerToPeerEndpoint {
    Creator,
    Joiner,
}

impl<'player> PeerToPeerNetwork<'player, PokerPeerToPeerConnector> {
    pub fn use_connector(connector: PokerPeerToPeerConnector) -> Self {
        Self {
            runtime: PokerPeerToPeerRuntime {
                connector,
                topology: TopologyPolicy::Ring,
                endpoint: None,
                player: None,
                chain: None,
            },
            connector_type: PhantomData,
        }
    }

    pub fn with_topology(mut self, topology: TopologyPolicy) -> Self {
        let runtime = self.runtime_mut();
        runtime.topology = topology;
        self
    }

    pub fn install_chain(
        &mut self,
        table_client: PrivatePokerTableClient,
        hand_client: PrivatePokerHandClient,
        signal_client: PrivatePokerSignalClient,
        wallet_address: alloy_primitives::Address,
        lobby_id: u64,
        table_id: u64,
    ) {
        self.runtime_mut().chain = Some(PokerPeerToPeerChain {
            hand_client,
            signal_client,
            table_client,
            wallet_address,
            lobby_id,
            table_id,
        });
    }

    pub fn prepare_table_creator(&mut self) {
        self.runtime_mut().endpoint = Some(PokerPeerToPeerEndpoint::Creator);
    }

    pub fn prepare_table_joiner(&mut self) {
        self.runtime_mut().endpoint = Some(PokerPeerToPeerEndpoint::Joiner);
    }

    pub async fn get_connection_info(&mut self) -> Result<Vec<u8>, String> {
        self.runtime_mut().get_connection_info().await
    }

    pub async fn connect_to_peer(&mut self, blob: Vec<u8>) -> Result<Vec<u8>, String> {
        self.runtime_mut().connect_to_peer(blob).await
    }

    pub fn register_player(&mut self, player: &'player mut PokerBot) -> Result<(), String> {
        self.runtime_mut().register_player(player)
    }

    pub async fn run(&mut self) -> Result<(), String> {
        self.runtime_mut().run().await
    }

    fn runtime_mut(&mut self) -> &mut PokerPeerToPeerRuntime<'player> {
        &mut self.runtime
    }

    fn runtime(&self) -> &PokerPeerToPeerRuntime<'player> {
        &self.runtime
    }
}

impl<'player> PokerPeerToPeerRuntime<'player> {
    async fn get_connection_info(&mut self) -> Result<Vec<u8>, String> {
        self.connector.get_connection_info().await
    }

    async fn connect_to_peer(&mut self, blob: Vec<u8>) -> Result<Vec<u8>, String> {
        self.connector.connect_to_peer(blob).await
    }

    fn register_player(&mut self, player: &'player mut PokerBot) -> Result<(), String> {
        if self.player.is_some() {
            return Err("p2p network already has a registered player".to_string());
        }
        self.player = Some(player);
        Ok(())
    }

    async fn run(&mut self) -> Result<(), String> {
        tracing::info!(
            "p2p WebRTC adapter ready: stun_server={}, topology={:?}, player_registered={}",
            self.connector.stun_server,
            self.topology,
            self.player.is_some()
        );

        let player_index = self.player()?.player_index()?;
        let chain = self
            .chain
            .clone()
            .ok_or_else(|| "p2p network has no chain signal context".to_string())?;
        let mut connector_messages = self.connector.take_messages()?;
        let mut provider_events = tokio::time::interval(Duration::from_millis(500));
        let shutdown = tokio::signal::ctrl_c();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                _ = provider_events.tick() => {
                    for event in chain
                        .table_client
                        .poll_player_joined(chain.lobby_id, chain.table_id)
                        .await?
                    {
                        tracing::info!("PlayerJoined player_address={}", event.player_address);
                        if self.player()?.should_publish_connection_info()? {
                            self.maybe_publish_connection_info(&chain).await?;
                        }
                    }

                    for signal in chain
                        .signal_client
                        .poll_handshake_signals(chain.lobby_id, chain.table_id)
                        .await?
                    {
                        self.handle_handshake_signal(&chain, signal, player_index).await?;
                    }

                    for event in chain
                        .hand_client
                        .poll_hand_started(chain.lobby_id, chain.table_id)
                        .await?
                    {
                        tracing::info!(
                            "HandStarted seat_number={} remain_count={}",
                            event.seat_number,
                            event.remain_count
                        );
                        if self.player_mut()?.on_hand_started(&event)? {
                            tracing::info!("Starting gameplay as Player {}", player_index + 1);
                        }
                    }
                }
                Some(message) = connector_messages.recv() => {
                    self.handle_connector_message(message).await?;
                }
            }

            if self.player()?.should_tick()? && self.tick_poker(player_index).await? {
                break;
            }
        }

        tokio::time::sleep(Duration::from_millis(750)).await;
        self.connector.close().await?;

        Ok(())
    }

    async fn handle_handshake_signal(
        &mut self,
        chain: &PokerPeerToPeerChain,
        signal: HandshakeSignalOnChain,
        player_index: usize,
    ) -> Result<(), String> {
        match self.connector.connect_to_peer_from_signal(&signal).await? {
            SignallingOutcome::SendAnswer(answer) => {
                let peer_public_key = self.peer_annonce_public_key(chain, signal.sender).await?;
                let encrypted_answer = encrypt_signal_for_peer(&peer_public_key, &answer)?;
                chain
                    .signal_client
                    .send_signal(
                        chain.lobby_id,
                        chain.table_id,
                        signal.sender,
                        encrypted_answer,
                    )
                    .await?;
            }
            SignallingOutcome::ConnectionEstablished => {
                chain
                    .hand_client
                    .start_hand(chain.lobby_id, chain.table_id)
                    .await?;
                self.player_mut()?.mark_connected()?;
                tracing::info!("called startHand for Player {}", player_index + 1);
            }
        }
        Ok(())
    }

    async fn maybe_publish_connection_info(
        &mut self,
        chain: &PokerPeerToPeerChain,
    ) -> Result<(), String> {
        let detail = chain
            .table_client
            .get_table_detail(chain.lobby_id, chain.table_id)
            .await?;
        let num_players = detail.info.flags.to::<usize>();
        if num_players == 0 {
            return Err("table has num_players=0; create it with --num-players".to_string());
        }
        if detail.players.len() < num_players {
            tracing::info!(
                "waiting for table to fill: seated={} num_players={}",
                detail.players.len(),
                num_players
            );
            return Ok(());
        }

        let seat_number = detail
            .players
            .iter()
            .position(|player| player.address == chain.wallet_address)
            .ok_or_else(|| "local wallet is not seated at table".to_string())?;
        let peer_index = if seat_number == 0 {
            detail.players.len() - 1
        } else {
            seat_number - 1
        };
        let peer = &detail.players[peer_index];
        let connection_info = self.connector.get_connection_info().await?;
        let encrypted_data =
            encrypt_signal_for_peer(&peer.annonce_public_key_hex, &connection_info)?;
        chain
            .signal_client
            .send_signal(chain.lobby_id, chain.table_id, peer.address, encrypted_data)
            .await?;
        self.player_mut()?.mark_connection_info_sent()?;
        tracing::info!(
            "sent WebRTC offer to previous ring player {}; ring update direction is Player {} => Player {}",
            peer.address,
            peer_index + 1,
            seat_number + 1
        );
        Ok(())
    }

    async fn peer_annonce_public_key(
        &self,
        chain: &PokerPeerToPeerChain,
        peer_address: alloy_primitives::Address,
    ) -> Result<String, String> {
        let detail = chain
            .table_client
            .get_table_detail(chain.lobby_id, chain.table_id)
            .await?;
        detail
            .players
            .into_iter()
            .find(|player| player.address == peer_address)
            .map(|player| player.annonce_public_key_hex)
            .ok_or_else(|| format!("HandshakeSignal sender {peer_address} is not seated"))
    }

    async fn handle_connector_message(&mut self, message: Vec<u8>) -> Result<(), String> {
        let wire: PokerActionEnvelopeWire =
            nson::decode::from_bytes(&message).map_err(|err| err.to_string())?;
        let envelope = wire.to_envelope()?;
        let applied = self
            .player_mut()?
            .client_state_mut()?
            .apply_envelope(envelope)
            .map_err(|err| format!("failed to apply poker action: 0x{}", hex::encode(err)))?;
        if applied {
            for line in wire.log_lines {
                println!("{line}");
            }
            self.connector.send(message).await?;
        }
        Ok(())
    }

    async fn tick_poker(&mut self, player_index: usize) -> Result<bool, String> {
        let acting_player =
            current_player(&self.player()?.client_state()?.table).map_err(|err| err.to_string())?;
        let can_send = self.connector.can_send();
        if self
            .player()?
            .should_act(acting_player, player_index, can_send)?
        {
            let (wire, log_lines) = self.call_player_act(&[PlayerEvent::Tick])?;
            for line in &log_lines {
                println!("{line}");
            }
            self.send_action(wire).await?;
        }
        if is_finished(&self.player()?.client_state()?.table).map_err(|err| err.to_string())? {
            for line in
                final_lines(&self.player()?.client_state()?.table).map_err(|err| err.to_string())?
            {
                println!("{line}");
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn player(&self) -> Result<&PokerBot, String> {
        self.player
            .as_deref()
            .ok_or_else(|| "p2p network has no registered player".to_string())
    }

    fn player_mut(&mut self) -> Result<&mut PokerBot, String> {
        self.player
            .as_deref_mut()
            .ok_or_else(|| "p2p network has no registered player".to_string())
    }

    async fn send_action(&mut self, wire: PokerActionEnvelopeWire) -> Result<(), String> {
        self.connector
            .send(nson::encode::to_bytes(&wire).map_err(|err| err.to_string())?)
            .await
    }

    fn call_player_act(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), String> {
        self.player_mut()?.act_current_game(events)
    }
}

impl<'player> PokerPlayerRegistrar<'player, PokerBot>
    for PeerToPeerNetwork<'player, PokerPeerToPeerConnector>
{
    type Error = String;

    fn register_player(&mut self, player: &'player mut PokerBot) -> Result<(), Self::Error> {
        PeerToPeerNetwork::register_player(self, player)
    }
}

impl PokerPeerToPeerDetails for PeerToPeerNetwork<'_, PokerPeerToPeerConnector> {
    fn annonce_public_key_hex(&self) -> &str {
        &self
            .runtime()
            .connector
            .player_details
            .annonce_public_key_hex
    }

    fn install_chain(
        &mut self,
        table_client: PrivatePokerTableClient,
        hand_client: PrivatePokerHandClient,
        signal_client: PrivatePokerSignalClient,
        wallet_address: alloy_primitives::Address,
        lobby_id: u64,
        table_id: u64,
    ) {
        PeerToPeerNetwork::install_chain(
            self,
            table_client,
            hand_client,
            signal_client,
            wallet_address,
            lobby_id,
            table_id,
        );
    }

    async fn get_connection_info(&mut self) -> Result<Vec<u8>, String> {
        PeerToPeerNetwork::get_connection_info(self).await
    }

    async fn connect_to_peer(&mut self, blob: Vec<u8>) -> Result<Vec<u8>, String> {
        PeerToPeerNetwork::connect_to_peer(self, blob).await
    }

    fn prepare_table_creator(&mut self) {
        PeerToPeerNetwork::prepare_table_creator(self);
    }

    fn prepare_table_joiner(&mut self) {
        PeerToPeerNetwork::prepare_table_joiner(self);
    }
}

fn attach_message_handler(data_channel: &Arc<RTCDataChannel>, message_tx: mpsc::Sender<Vec<u8>>) {
    data_channel.on_message(Box::new(move |message: DataChannelMessage| {
        let message_tx = message_tx.clone();
        Box::pin(async move {
            let _ = message_tx.send(message.data.to_vec()).await;
        })
    }));
}

async fn wait_data_channel_open(data_channel: &RTCDataChannel) -> Result<(), String> {
    for _ in 0..100 {
        if data_channel.ready_state() == RTCDataChannelState::Open {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err("WebRTC data channel did not open after answer".to_string())
}
