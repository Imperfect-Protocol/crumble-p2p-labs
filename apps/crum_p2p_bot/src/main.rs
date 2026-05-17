//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::PokerBot;
use crum_p2p_bot::{
    config::{BotConfig, BotMode, CliOptions, usage},
    identity::derive_bls_identity,
    lobby::{PokerArbitrumDetails, PokerArbitrumLobby},
    network::{PokerPeerToPeerConnector, PokerPeerToPeerNetwork, PokerPeerToPeerPlayerDetails},
    table_registry::{default_registry_path, remember_table, resolve_table},
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
        },
    );
    let network = PokerPeerToPeerNetwork::use_connector(connector)
        .with_topology(config.topology, config.max_players);
    let details =
        PokerArbitrumDetails::new(config.rpc_url.clone(), private_key, lobby_address, lobby_id);
    let mut lobby = PokerArbitrumLobby::use_network(network, details).await?;

    match config.mode {
        BotMode::Create => {
            let table_name = config.table_name.as_deref().ok_or_else(|| {
                "missing table name; pass --create-table NAME or --table-name NAME".to_string()
            })?;
            let buy_in = config.buy_in.unwrap_or(0);
            let created = lobby.create_table(table_name, buy_in).await?;
            println!(
                "Created table id={} lobby={}",
                created.table_id, created.lobby_id
            );
            if let Err(err) = remember_table(&default_registry_path(), table_name, created.table_id)
            {
                tracing::warn!("failed to remember table name `{table_name}`: {err}");
            }
        }
        BotMode::Join => {
            let table_id = resolve_join_table_id(&config)?;
            let joined = lobby.join_table(table_id).await?;
            println!(
                "Joined table id={} lobby={}",
                joined.table_id, joined.lobby_id
            );
        }
    }

    let mut network = lobby.into_network();
    bot.handle_actions(&mut network)?;
    network.run().await?;

    Ok(())
}

fn resolve_join_table_id(config: &BotConfig) -> Result<u64, String> {
    if let Some(table_id) = config.table_id {
        return Ok(table_id);
    }

    let Some(table_name) = config.table_name.as_deref() else {
        return Err(
            "missing table id or name; pass --join-table TABLE_ID_OR_NAME or --table-id TABLE_ID"
                .to_string(),
        );
    };

    resolve_table(&default_registry_path(), table_name)?.ok_or_else(|| {
        format!(
            "table `{table_name}` is not in {}; create it first in this working directory or pass --table-id",
            default_registry_path().display()
        )
    })
}
