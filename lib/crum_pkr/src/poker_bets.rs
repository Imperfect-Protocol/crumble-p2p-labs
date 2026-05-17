//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

#[derive(Clone, Debug)]
pub struct PokerBettingState {
    player_chips: Vec<u64>,
    current_round_bets: Vec<Option<u64>>,
    pot: u64,
    active_players: Vec<bool>,
    current_highest_bet: u64,
}

impl PokerBettingState {
    pub fn new(num_players: usize, initial_chips: u64) -> Self {
        Self {
            player_chips: vec![initial_chips; num_players],
            current_round_bets: vec![None; num_players],
            pot: 0,
            active_players: vec![true; num_players],
            current_highest_bet: 0,
        }
    }

    pub fn call_amount_required(&self, player: usize) -> Result<u64, Vec<u8>> {
        if !self.active_players[player] {
            return Err(b"Player has already folded".to_vec());
        }

        let amount_needed_to_call =
            self.current_highest_bet - self.current_round_bets[player].unwrap_or(0);

        Ok(amount_needed_to_call)
    }

    pub fn chips_remaining(&self, player: usize) -> u64 {
        self.player_chips[player]
    }

    pub fn get_active_players(&self) -> &Vec<bool> {
        &self.active_players
    }

    /// Process a player's betting action based purely on the amount of chips put in.
    /// amount = 0 means Check (if no bet to call) or Fold (if facing a bet).
    /// amount > 0 means Call or Raise.
    pub fn process_action(&mut self, player: usize, amount: u64) -> Result<(), Vec<u8>> {
        if !self.active_players[player] {
            return Err(b"Player has already folded".to_vec());
        }

        // How much this player needs to put in to stay in the hand
        let amount_needed_to_call =
            self.current_highest_bet - self.current_round_bets[player].unwrap_or(0);

        if amount == 0 {
            if amount_needed_to_call > 0 {
                // They owe chips but put in 0. This is a Fold.
                self.active_players[player] = false;
            } else {
                // They owe nothing and put in 0. This is a Check.
                self.current_round_bets[player] = Some(0);
            }
        } else {
            // They are putting chips in. Verify it's legal.
            if amount < amount_needed_to_call {
                return Err(b"Amount is less than the required call amount".to_vec());
                // Note: True all-in rules (putting in less than the call amount because
                // the stack is empty) would be handled right here.
            }

            if self.player_chips[player] < amount {
                return Err(b"Not enough chips in stack".to_vec());
            }

            // Move chips from player stack to the pot
            self.player_chips[player] -= amount;
            self.current_round_bets[player] =
                Some(amount + self.current_round_bets[player].unwrap_or(0));
            self.pot += amount;

            // If they put in more than what was needed to call, it's a raise.
            // Update the new highest bet for everyone else to match.
            if amount > amount_needed_to_call {
                self.current_highest_bet = self.current_round_bets[player].unwrap_or(0);
            }
        }

        Ok(())
    }

    pub fn is_betting_round_complete(&self) -> bool {
        let active_count = self.active_players.iter().filter(|&&active| active).count();

        // If only one person is left, the hand is effectively over
        if active_count <= 1 {
            return true;
        }

        // The round is complete when every active player's current bet matches the highest bet
        for (player, &is_active) in self.active_players.iter().enumerate() {
            if !is_active {
                continue;
            }
            let Some(player_bet) = self.current_round_bets[player] else {
                return false;
            };
            if player_bet < self.current_highest_bet {
                return false;
            }
        }

        true
    }

    /// Resets the street-level tracking variables for the next round (Flop, Turn, River)
    pub fn next_street(&mut self) {
        self.current_round_bets.fill(None);
        self.current_highest_bet = 0;
    }
}
