//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::{
    Player, PlayerEvent, PokerAction, PokerActionEnvelopeWire, PokerBotLogic, PokerBotLogicError,
    PokerClientState, PokerGameState, current_player, final_lines, is_finished,
};
use crum_pkr::{poker_state::POKER_HOLDEM_ROUNDS, poker_table::PokerTable};
use rand::{Rng, distributions::Uniform, thread_rng};

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

    pub fn client_state(&self) -> &PokerClientState {
        &self.session.state
    }

    pub fn client_state_mut(&mut self) -> &mut PokerClientState {
        &mut self.session.state
    }

    pub fn act(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), PokerBotLogicError> {
        let mut state = InMemoryPokerGameState::new(&mut self.session.state);
        self.logic.act(&mut state, events)?;
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

    pub fn current_game(&self) -> Result<&PokerBotGame, PokerBotLogicError> {
        self.games
            .first()
            .ok_or_else(|| PokerBotLogicError::from("bot has no poker game"))
    }

    pub fn current_game_mut(&mut self) -> Result<&mut PokerBotGame, PokerBotLogicError> {
        self.games
            .first_mut()
            .ok_or_else(|| PokerBotLogicError::from("bot has no poker game"))
    }

    pub fn client_state(&self) -> Result<&PokerClientState, PokerBotLogicError> {
        Ok(self.current_game()?.client_state())
    }

    pub fn client_state_mut(&mut self) -> Result<&mut PokerClientState, PokerBotLogicError> {
        Ok(self.current_game_mut()?.client_state_mut())
    }

    pub fn act_current_game(
        &mut self,
        events: &[PlayerEvent],
    ) -> Result<(PokerActionEnvelopeWire, Vec<String>), PokerBotLogicError> {
        self.current_game_mut()?.act(events)
    }
}

impl<State> Player<State, PlayerEvent> for PokerBot
where
    State: PokerGameState<Error = PokerBotLogicError>,
{
    type Error = PokerBotLogicError;

    fn act(&mut self, state: &mut State, events: &[PlayerEvent]) -> Result<(), Self::Error> {
        self.current_game_mut()?.logic.act(state, events)
    }
}

pub struct InMemoryNetwork<'player> {
    players: Vec<&'player mut PokerBot>,
}

impl<'player> InMemoryNetwork<'player> {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
        }
    }

    pub fn register_player(&mut self, player: &'player mut PokerBot) {
        self.players.push(player);
    }

    pub fn run(&mut self) -> Result<(), PokerBotLogicError> {
        loop {
            let table = &self
                .players
                .first()
                .ok_or_else(|| PokerBotLogicError::from("in-memory network has no players"))?
                .client_state()?
                .table;
            if is_finished(table)? {
                for line in final_lines(table)? {
                    println!("{line}");
                }
                break;
            }

            let player_index = current_player(table)?;
            let Some(player) = self.players.get_mut(player_index) else {
                return Err(PokerBotLogicError::from(format!(
                    "no in-memory player registered for Player {}",
                    player_index + 1
                )));
            };
            let (wire, log_lines) = player.act_current_game(&[PlayerEvent::Tick])?;
            for line in log_lines {
                println!("{line}");
            }
            let envelope = wire.to_envelope()?;
            for (index, peer) in self.players.iter_mut().enumerate() {
                if index != player_index {
                    peer.client_state_mut()?.apply_envelope(envelope.clone())?;
                    peer.client_state_mut()?.player_index = index;
                }
            }
        }
        Ok(())
    }
}

pub struct InMemoryPokerGameState<'state> {
    client_state: &'state mut PokerClientState,
    wire: Option<PokerActionEnvelopeWire>,
    log_lines: Vec<String>,
}

impl<'state> InMemoryPokerGameState<'state> {
    fn new(client_state: &'state mut PokerClientState) -> Self {
        Self {
            client_state,
            wire: None,
            log_lines: Vec::new(),
        }
    }

    fn into_wire(self) -> Result<(PokerActionEnvelopeWire, Vec<String>), PokerBotLogicError> {
        let wire = self
            .wire
            .ok_or_else(|| PokerBotLogicError::from("player did not submit an action"))?;
        Ok((wire, self.log_lines))
    }
}

impl PokerGameState for InMemoryPokerGameState<'_> {
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

pub fn run(
    num_players: usize,
    initial_chips: u64,
    small_blind: u64,
) -> Result<(), PokerBotLogicError> {
    let mut poker_table = PokerTable::new(num_players, POKER_HOLDEM_ROUNDS);
    for player_id in 1..=num_players {
        poker_table.join(player_id as u32);
    }
    poker_table.start_hand(initial_chips, small_blind)?;

    let mut bots = (0..num_players)
        .map(|player_index| {
            let mut player_table = PokerTable::new(num_players, POKER_HOLDEM_ROUNDS);
            for player_id in 1..=num_players {
                player_table.join(player_id as u32);
            }
            player_table.start_hand(initial_chips, small_blind)?;
            let mut bot = PokerBot::new();
            bot.track_game(
                PokerClientState::new(player_index, 1, 1, player_table),
                player_index,
            );
            Ok(bot)
        })
        .collect::<Result<Vec<_>, PokerBotLogicError>>()?;

    let mut network = InMemoryNetwork::new();

    for bot in &mut bots {
        network.register_player(bot);
    }

    network.run()
}

fn init_logging() {
    if cfg!(feature = "pure_output") {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_level(false)
            .without_time()
            .init();
    } else {
        tracing_subscriber::fmt::init();
    }
}

pub fn main() {
    init_logging();

    #[cfg(not(feature = "six_player"))]
    let num_players = thread_rng().sample(Uniform::new_inclusive(2usize, 6usize));

    #[cfg(feature = "six_player")]
    let num_players = 6;

    let initial_chips = 1000;
    let small_blind = 10;

    if let Err(err) = run(num_players, initial_chips, small_blind) {
        tracing::error!("Error: {err}");
    }
}
