//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_pkr::poker_table::PokerTable;

use crate::{
    Player, PlayerEvent, PokerAction, PokerBotError, PokerClientState, PokerGameState,
    PokerNetwork, PokerPlayerRegistrar, current_player, final_lines, is_finished,
};

type InMemoryPlayerCallback<'player> = Box<
    dyn FnMut(&mut PokerClientState, &[PlayerEvent]) -> Result<Vec<String>, PokerBotError>
        + 'player,
>;

pub struct InMemoryNetwork<'player> {
    client_state: Option<PokerClientState>,
    players: Vec<InMemoryPlayerCallback<'player>>,
}

impl<'player> InMemoryNetwork<'player> {
    pub fn new() -> Self {
        Self {
            client_state: None,
            players: Vec::new(),
        }
    }

    pub fn register_player<P>(&mut self, player: &'player mut P) -> Result<(), PokerBotError>
    where
        P: for<'state> Player<InMemoryPokerGameState<'state>, PlayerEvent, Error = PokerBotError>
            + 'player,
    {
        self.players.push(Box::new(move |client_state, events| {
            let mut game_state = InMemoryPokerGameState::new(client_state);
            player.act(&mut game_state, events)?;
            game_state.into_log_lines()
        }));
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), PokerBotError> {
        loop {
            if is_finished(&self.client_state()?.table)? {
                for line in final_lines(&self.client_state()?.table)? {
                    println!("{line}");
                }
                break;
            }

            let player_index = current_player(&self.client_state()?.table)?;
            let client_state = self
                .client_state
                .as_mut()
                .ok_or_else(|| PokerBotError::from("in-memory network has no poker state"))?;
            let Some(player) = self.players.get_mut(player_index) else {
                return Err(PokerBotError::from(format!(
                    "no in-memory player registered for Player {}",
                    player_index + 1
                )));
            };

            client_state.player_index = player_index;
            for line in player(client_state, &[PlayerEvent::Tick])? {
                println!("{line}");
            }
        }

        Ok(())
    }

    fn client_state(&self) -> Result<&PokerClientState, PokerBotError> {
        self.client_state
            .as_ref()
            .ok_or_else(|| PokerBotError::from("in-memory network has no poker state"))
    }
}

impl Default for InMemoryNetwork<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl PokerNetwork for InMemoryNetwork<'_> {
    type Error = PokerBotError;

    fn install_poker_state(&mut self, state: PokerClientState) -> Result<(), Self::Error> {
        self.client_state = Some(state);
        Ok(())
    }

    fn mark_table_ready(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'player, P> PokerPlayerRegistrar<'player, P> for InMemoryNetwork<'player>
where
    P: for<'state> Player<InMemoryPokerGameState<'state>, PlayerEvent, Error = PokerBotError>
        + 'player,
{
    type Error = PokerBotError;

    fn register_player(&mut self, player: &'player mut P) -> Result<(), Self::Error> {
        InMemoryNetwork::register_player(self, player)
    }
}

pub struct InMemoryPokerGameState<'state> {
    client_state: &'state mut PokerClientState,
    log_lines: Vec<String>,
}

impl<'state> InMemoryPokerGameState<'state> {
    fn new(client_state: &'state mut PokerClientState) -> Self {
        Self {
            client_state,
            log_lines: Vec::new(),
        }
    }

    fn into_log_lines(self) -> Result<Vec<String>, PokerBotError> {
        Ok(self.log_lines)
    }
}

impl PokerGameState for InMemoryPokerGameState<'_> {
    type Error = PokerBotError;

    fn table(&self) -> &PokerTable {
        &self.client_state.table
    }

    fn submit_action(
        &mut self,
        action: PokerAction,
        log_lines: Vec<String>,
    ) -> Result<(), Self::Error> {
        self.client_state.submit_local_action(action)?;
        self.log_lines = log_lines;
        Ok(())
    }
}
