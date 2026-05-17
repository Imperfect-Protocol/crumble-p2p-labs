//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_net::{InMemoryNetwork, PokerBot, PokerBotError, PokerClientState, PokerNetwork};
use crum_pkr::{poker_state::POKER_HOLDEM_ROUNDS, poker_table::PokerTable};
use rand::{Rng, distributions::Uniform, thread_rng};

pub fn run(num_players: usize, initial_chips: u64, small_blind: u64) -> Result<(), PokerBotError> {
    let mut poker_table = PokerTable::new(num_players, POKER_HOLDEM_ROUNDS);
    for player_id in 1..=num_players {
        poker_table.join(player_id as u32);
    }
    poker_table.start_hand(initial_chips, small_blind)?;

    let mut bots = (0..num_players)
        .map(|player_index| {
            let mut bot = PokerBot::new();
            bot.seat_as(player_index);
            bot
        })
        .collect::<Vec<_>>();

    let mut network = InMemoryNetwork::new();
    network.install_poker_state(PokerClientState::new(0, 1, 1, poker_table))?;

    for bot in &mut bots {
        bot.handle_actions(&mut network)?;
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
