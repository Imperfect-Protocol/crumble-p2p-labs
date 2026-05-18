//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::{
    Player, PlayerEvent, PokerActionEnvelopeWire, PokerBotLogicError, PokerClientState,
    PokerGameClientState, PokerGameState, PokerPlayerRegistrar,
};

use crate::chain::HandStartedOnChain;
use crate::game::PokerBotGame;

pub struct PokerBot {
    games: Vec<PokerBotGame>,
}

impl PokerBot {
    pub fn new() -> Self {
        Self { games: Vec::new() }
    }

    pub fn games(&self) -> &[PokerBotGame] {
        &self.games
    }

    pub fn games_mut(&mut self) -> &mut [PokerBotGame] {
        &mut self.games
    }

    pub fn current_game(&self) -> Result<&PokerBotGame, String> {
        self.games
            .first()
            .ok_or_else(|| "bot has no poker game".to_string())
    }

    pub fn current_game_mut(&mut self) -> Result<&mut PokerBotGame, String> {
        self.games
            .first_mut()
            .ok_or_else(|| "bot has no poker game".to_string())
    }

    pub fn player_index(&self) -> Result<usize, String> {
        Ok(self.current_game()?.player_index())
    }

    pub fn client_state(&self) -> Result<&PokerClientState, String> {
        Ok(self.current_game()?.client_state())
    }

    pub fn client_state_mut(&mut self) -> Result<&mut PokerClientState, String> {
        Ok(self.current_game_mut()?.client_state_mut())
    }

    pub fn should_publish_connection_info(&self) -> Result<bool, String> {
        Ok(self.current_game()?.should_publish_connection_info())
    }

    pub fn mark_connection_info_sent(&mut self) -> Result<(), String> {
        self.current_game_mut()?.mark_connection_info_sent();
        Ok(())
    }

    pub fn mark_connected(&mut self) -> Result<(), String> {
        self.current_game_mut()?.mark_connected();
        Ok(())
    }

    pub fn on_hand_started(&mut self, event: &HandStartedOnChain) -> Result<bool, String> {
        Ok(self.current_game_mut()?.on_hand_started(event))
    }

    pub fn should_tick(&self) -> Result<bool, String> {
        Ok(self.current_game()?.should_tick())
    }

    pub fn should_act(
        &self,
        acting_player: usize,
        player_index: usize,
        can_send: bool,
    ) -> Result<bool, String> {
        Ok(self
            .current_game()?
            .should_act(acting_player, player_index, can_send))
    }

    pub fn act_current_game(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), String> {
        self.current_game_mut()?.act(events)
    }

    pub fn track_arbitrum_game(
        &mut self,
        state: PokerGameClientState,
        lobby_id: u64,
        table_id: u64,
        player_index: usize,
    ) {
        self.games.push(PokerBotGame::for_arbitrum(
            state,
            lobby_id,
            table_id,
            player_index,
        ));
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

impl Default for PokerBot {
    fn default() -> Self {
        Self::new()
    }
}

impl<State> Player<State, PlayerEvent> for PokerBot
where
    State: PokerGameState<Error = PokerBotLogicError>,
{
    type Error = PokerBotLogicError;

    fn act(&mut self, state: &mut State, events: &[PlayerEvent]) -> Result<(), Self::Error> {
        let game = self
            .games
            .first_mut()
            .ok_or_else(|| PokerBotLogicError::from("bot has no poker game"))?;
        game.logic_mut().act(state, events)
    }
}

impl crum_net::BotGame for PokerBotGame {}
