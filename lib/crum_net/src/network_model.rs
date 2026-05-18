//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crate::{Player, PokerClientState, PokerGameEvent, PokerGameState};

pub trait NetworkConnector {}

#[allow(async_fn_in_trait)]
pub trait PokerLobby {
    type Error;

    async fn run(&mut self) -> Result<(), Self::Error>;
}

pub trait PokerGame {
    type Session: PokerGameSession;

    fn session(&self) -> &Self::Session;

    fn session_mut(&mut self) -> &mut Self::Session;
}

pub trait PokerGameSession {
    type Connector: NetworkConnector;

    fn state(&self) -> &PokerClientState;

    fn state_mut(&mut self) -> &mut PokerClientState;

    fn connector(&self) -> &Self::Connector;

    fn connector_mut(&mut self) -> &mut Self::Connector;
}

pub struct Network<Connector, Lobby, Game, PlayerT>
where
    Connector: NetworkConnector,
    Lobby: PokerLobby,
    Game: PokerGame,
{
    pub connector: Connector,
    pub lobby: Lobby,
    pub games: Vec<Game>,
    pub players: Vec<PlayerT>,
}

pub trait BotGame {}

pub trait Bot<State, Event>
where
    State: PokerGameState,
    Event: PokerGameEvent,
{
    type Player: Player<State, Event>;

    fn games(&self) -> &[Box<dyn BotGame>];

    fn games_mut(&mut self) -> &mut [Box<dyn BotGame>];
}
