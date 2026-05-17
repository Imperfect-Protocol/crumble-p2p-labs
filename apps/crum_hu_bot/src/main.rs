//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

mod lobby;
mod network;

use std::env;

use crum_net::PokerBot;
use lobby::PokerClientServerLobby;
use network::{PokerClient, PokerClientServerNetwork, PokerServer};

#[derive(Debug)]
enum Mode {
    Listen,
    Connect,
}

#[derive(Debug)]
struct Cli {
    mode: Mode,
    host: String,
    port: u16,
}

#[tokio::main]
async fn main() {
    init_logging();

    if let Err(err) = async_main().await {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_cli(env::args().skip(1))?;
    let mode = cli.mode;
    let mut bot = PokerBot::new();
    let network = match mode {
        Mode::Listen => {
            PokerClientServerNetwork::use_server(PokerServer::new(cli.host, cli.port)).await?
        }
        Mode::Connect => {
            PokerClientServerNetwork::use_client(PokerClient::new(cli.host, cli.port)).await?
        }
    };

    let mut lobby = PokerClientServerLobby::use_network(network);
    match mode {
        Mode::Listen => {
            bot.seat_as(0);
            lobby.create_table(0)?;
        }
        Mode::Connect => {
            bot.seat_as(1);
            lobby.join_table(1)?;
        }
    }

    let mut network = lobby.into_network();
    bot.handle_actions(&mut network)?;
    network.run().await?;
    Ok(())
}

fn parse_cli(args: impl IntoIterator<Item = String>) -> Result<Cli, String> {
    let mut mode = None;
    let mut host = None;
    let mut port = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--listen" => {
                mode = Some(Mode::Listen);
                host = args.next();
            }
            "--connect" => {
                mode = Some(Mode::Connect);
                host = args.next();
            }
            "--port" | "-port" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value after --port".to_string())?;
                port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|_| format!("invalid port: {value}"))?,
                );
            }
            "--help" | "-h" => return Err(usage()),
            other => return Err(format!("unexpected argument: {other}\n{}", usage())),
        }
    }

    Ok(Cli {
        mode: mode.ok_or_else(usage)?,
        host: host.ok_or_else(usage)?,
        port: port.unwrap_or(3000),
    })
}

fn usage() -> String {
    "usage: crum_hu_bot (--listen HOST | --connect HOST) [--port PORT|-port PORT]".to_string()
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_level(true)
        .compact()
        .init();
}
