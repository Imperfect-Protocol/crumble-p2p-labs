//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crate::poker_hand::PokerHand;

pub struct PokerTable {
    max_players: usize,
    max_rounds: usize,
    current_players: Vec<u32>,
    dealer_button: usize,
    current_hand: Option<PokerHand>,
}

impl PokerTable {
    /// Player 1 creates a table
    pub fn new(max_players: usize, max_rounds: usize) -> Self {
        Self {
            max_players,
            max_rounds,
            current_players: vec![],
            dealer_button: 0,
            current_hand: None,
        }
    }

    /// Player 1, 2 (3,4,...) joins a table
    pub fn join(&mut self, player: u32) {
        // check player already joined
        self.current_players.push(player);
        // emit player joined
    }

    /// Player 1 starts new hand (at their discretion) with players at the table
    pub fn start_hand(&mut self, initial_chips: u64, small_blind: u64) -> Result<(), Vec<u8>> {
        // check player 1 is submitter
        // check hand in progress

        if !self
            .current_hand
            .as_ref()
            .is_none_or(|h| h.get_current_state().is_finished())
        {
            return Err(b"Hand in progress")?;
        }

        self.current_hand.replace(PokerHand::new(
            self.current_players.len(),
            self.max_rounds,
            self.dealer_button,
            initial_chips,
            small_blind,
        ));

        // emit hand started

        Ok(())
    }

    /// Supports gameplay
    pub const fn get_current_hand(&self) -> Option<&PokerHand> {
        self.current_hand.as_ref()
    }

    /// Supports gameplay
    pub const fn get_current_hand_mut(&mut self) -> Option<&mut PokerHand> {
        self.current_hand.as_mut()
    }

    pub const fn get_max_players(&self) -> usize {
        self.max_players
    }

    pub const fn get_max_rounds(&self) -> usize {
        self.max_rounds
    }

    pub const fn get_current_player_count(&self) -> usize {
        self.current_players.len()
    }

    pub fn get_player(&self, player: usize) -> Option<u32> {
        self.current_players.get(player).cloned()
    }
}
