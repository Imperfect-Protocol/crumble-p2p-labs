//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_p2p_bot::{
    bot::PokerBot,
    config::{BotConfig, BotMode, CliOptions, usage},
    identity::derive_bls_identity,
    lobby::{PokerArbitrumBotExt, PokerArbitrumDetails, PokerArbitrumLobby},
    network::{PeerToPeerNetwork, PokerPeerToPeerConnector, PokerPeerToPeerPlayerDetails},
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let cli = CliOptions::from_env().map_err(|err| {
        if err.starts_with("Usage:") {
            err
        } else {
            format!("{err}\n\n{}", usage())
        }
    })?;
    let config = BotConfig::load(cli)?;

    let Some(private_key) = config.private_key.as_deref() else {
        return Err(
            "missing Ethereum private key; pass --private-key or configure wallet.private_key"
                .to_string(),
        );
    };
    let mut bot = PokerBot::new();
    let bls_identity = derive_bls_identity(private_key)?;
    tracing::info!(
        "derived BLS annonce public key for connector: 0x{}",
        bls_identity.public_key_hex
    );
    tracing::info!(
        "mode={:?}, lobby_id={:?}, table_id={:?}, table_name={:?}, buy_in={:?}",
        config.mode,
        config.lobby_id,
        config.table_id,
        config.table_name,
        config.buy_in
    );
    tracing::info!(
        "contracts: lobby={:?}, signal={:?}, verify_shuffle={:?}, verify_unmasking={:?}",
        config.contracts.lobby,
        config.contracts.signal,
        config.contracts.verify_shuffle,
        config.contracts.verify_unmasking
    );

    let lobby_id = config
        .lobby_id
        .ok_or_else(|| "missing lobby id; pass --lobby or --lobby-id".to_string())?;
    let lobby_address = config.contracts.lobby.as_deref().ok_or_else(|| {
        "missing lobby contract address; configure contracts.lobby or pass --lobby-address"
            .to_string()
    })?;

    let connector = PokerPeerToPeerConnector::new(
        "stun:stun.l.google.com:19302",
        PokerPeerToPeerPlayerDetails {
            annonce_public_key_hex: bls_identity.public_key_hex,
            annonce_private_key_hex: bls_identity.private_key_hex,
        },
    );
    let network = PeerToPeerNetwork::use_connector(connector).with_topology(config.topology);
    let details =
        PokerArbitrumDetails::new(config.rpc_url.clone(), private_key, lobby_address, lobby_id)
            .with_signal_address(config.contracts.signal.clone());
    let mut lobby = PokerArbitrumLobby::use_network(network, details).await?;

    match config.mode {
        BotMode::Create => {
            let table_name = config.table_name.as_deref().ok_or_else(|| {
                "missing table name; pass --table-name NAME with --create-table TABLE_TOML_PATH"
                    .to_string()
            })?;
            let buy_in = config.buy_in.unwrap_or(0);
            let num_players = config
                .num_players
                .ok_or_else(|| "missing num_players; set table.num_players in the table TOML or pass --num-players".to_string())?;
            let created = bot
                .create_table(&mut lobby, table_name, buy_in, num_players)
                .await?;
            println!(
                "Created table id={} lobby={}",
                created.table_id, created.lobby_id
            );
        }
        BotMode::Join => {
            let joined = if let Some(table_id) = config.table_id {
                bot.join_table(&mut lobby, table_id).await?
            } else {
                let table_name = config.table_name.as_deref().ok_or_else(|| {
                    "missing table name; pass --join-table --table-name TABLE_NAME --lobby-id LOBBY_ID"
                        .to_string()
                })?;
                bot.join_table_by_name(&mut lobby, table_name).await?
            };
            println!(
                "Joined table id={} lobby={}",
                joined.table_id, joined.lobby_id
            );
        }
    }

    lobby.run().await?;

    Ok(())
}
