//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

mod lobby;
mod network;

use std::env;

use crum_net::{
    Player, PlayerEvent, PokerAction, PokerActionEnvelopeWire, PokerBotLogic, PokerBotLogicError,
    PokerClientState, PokerGameState, PokerPlayerRegistrar,
};
use crum_pkr::poker_table::PokerTable;
use lobby::PokerClientServerLobby;
use network::{NetworkError, PokerClient, PokerClientServerNetwork, PokerServer};

pub struct PokerGameSession {
    state: PokerClientState,
}

pub struct PokerBotGame {
    session: PokerGameSession,
    logic: PokerBotLogic,
}

impl PokerBotGame {
    pub fn new(state: PokerClientState, player_index: usize) -> Self {
        let mut logic = PokerBotLogic::new();
        logic.seat_as(player_index);
        Self {
            session: PokerGameSession { state },
            logic,
        }
    }

    pub fn player_index(&self) -> usize {
        self.session.state.player_index
    }

    pub fn client_state(&self) -> &PokerClientState {
        &self.session.state
    }

    pub fn client_state_mut(&mut self) -> &mut PokerClientState {
        &mut self.session.state
    }

    pub fn act(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), NetworkError> {
        let mut state = PokerBotGameState::new(&mut self.session.state);
        self.logic
            .act(&mut state, events)
            .map_err(NetworkError::from)?;
        state.into_wire()
    }
}

pub struct PokerBot {
    games: Vec<PokerBotGame>,
}

impl PokerBot {
    pub fn new() -> Self {
        Self { games: Vec::new() }
    }

    pub fn track_game(&mut self, state: PokerClientState, player_index: usize) {
        self.games.push(PokerBotGame::new(state, player_index));
    }

    pub fn current_game(&self) -> Result<&PokerBotGame, NetworkError> {
        self.games
            .first()
            .ok_or_else(|| NetworkError::from("bot has no poker game"))
    }

    pub fn current_game_mut(&mut self) -> Result<&mut PokerBotGame, NetworkError> {
        self.games
            .first_mut()
            .ok_or_else(|| NetworkError::from("bot has no poker game"))
    }

    pub fn player_index(&self) -> Result<usize, NetworkError> {
        Ok(self.current_game()?.player_index())
    }

    pub fn client_state(&self) -> Result<&PokerClientState, NetworkError> {
        Ok(self.current_game()?.client_state())
    }

    pub fn client_state_mut(&mut self) -> Result<&mut PokerClientState, NetworkError> {
        Ok(self.current_game_mut()?.client_state_mut())
    }

    pub fn act_current_game(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), NetworkError> {
        self.current_game_mut()?.act(events)
    }

    pub fn handle_actions<'player, Network>(
        &'player mut self,
        network: &mut Network,
    ) -> Result<(), Network::Error>
    where
        Network: PokerPlayerRegistrar<'player, Self>,
    {
        network.register_player(self)
    }
}

impl<State> Player<State, PlayerEvent> for PokerBot
where
    State: PokerGameState<Error = PokerBotLogicError>,
{
    type Error = PokerBotLogicError;

    fn act(&mut self, state: &mut State, events: &[PlayerEvent]) -> Result<(), Self::Error> {
        self.current_game_mut()
            .map_err(|err| PokerBotLogicError::from(err.to_string()))?
            .logic
            .act(state, events)
    }
}

pub struct PokerBotGameState<'state> {
    client_state: &'state mut PokerClientState,
    wire: Option<PokerActionEnvelopeWire>,
    log_lines: Vec<String>,
}

impl<'state> PokerBotGameState<'state> {
    fn new(client_state: &'state mut PokerClientState) -> Self {
        Self {
            client_state,
            wire: None,
            log_lines: Vec::new(),
        }
    }

    fn into_wire(self) -> Result<(PokerActionEnvelopeWire, Vec<String>), NetworkError> {
        let wire = self
            .wire
            .ok_or_else(|| NetworkError::from("player did not submit an action"))?;
        Ok((wire, self.log_lines))
    }
}

impl PokerGameState for PokerBotGameState<'_> {
    type Error = PokerBotLogicError;

    fn table(&self) -> &PokerTable {
        &self.client_state.table
    }

    fn submit_action(
        &mut self,
        action: PokerAction,
        log_lines: Vec<String>,
    ) -> Result<(), Self::Error> {
        let envelope = self.client_state.submit_local_action(action)?;
        self.wire = Some(envelope.to_wire(log_lines.clone()));
        self.log_lines = log_lines;
        Ok(())
    }
}

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
    let (state, player_index) = match mode {
        Mode::Listen => (lobby.create_table(0)?, 0),
        Mode::Connect => (lobby.join_table(1)?, 1),
    };
    bot.track_game(state, player_index);

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
