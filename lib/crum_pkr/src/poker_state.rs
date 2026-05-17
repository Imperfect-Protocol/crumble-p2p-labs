//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

pub const POKER_HAND_STATE_SHUFFLE: u8 = 0;
pub const POKER_HAND_STATE_SMALL_BLIND: u8 = 1;
pub const POKER_HAND_STATE_BIG_BLIND: u8 = 2;
pub const POKER_HAND_STATE_BET: u8 = 3;
pub const POKER_HAND_STATE_UNMASK_HOLE_CARDS: u8 = 4;
pub const POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS: u8 = 5;
pub const POKER_HAND_STATE_UNMASK_SHOWDOWN: u8 = 6;
pub const POKER_HAND_STATE_SUBMIT_PUBLIC_KEY: u8 = 7;
pub const POKER_HAND_STATE_FINISHED: u8 = 8;
pub const POKER_HAND_STATE_CHEATED: u8 = 9;

pub const POKER_HOLDEM_PREFLOP: usize = 0;
pub const POKER_HOLDEM_FLOP: usize = 1;
pub const POKER_HOLDEM_TURN: usize = 2;
pub const POKER_HOLDEM_RIVER: usize = 3;
pub const POKER_HOLDEM_ROUNDS: usize = 4;

pub enum PokerHandStateEnum {
    Shuffle { player: usize, is_dealer: bool },
    SmallBlind { player: usize },
    BigBlind { player: usize },
    Bet { round: usize, player: usize },
    UnmaskHoleCards { player: usize },
    UnmaskCommunityCards { round: usize, player: usize },
    UnmaskShowdown { player: usize },
    SubmitPublicKey { player: usize },
    Cheated { player: usize },
    Finished,
    Invalid,
}

pub struct PokerHandState {
    pub(super) dealer_button: usize,
    pub(super) num_players: usize,
    pub(super) max_rounds: usize,
    pub(super) current_player: usize,
    pub(super) current_round: usize,
    pub(super) current_state: u8,
}

impl PokerHandState {
    pub const fn new(num_players: usize, max_rounds: usize, dealer_button: usize) -> Self {
        Self {
            num_players,
            max_rounds,
            dealer_button,
            current_player: dealer_button,
            current_round: 0,
            current_state: POKER_HAND_STATE_SHUFFLE,
        }
    }

    pub const fn is_dealer(&self, player: usize) -> bool {
        self.dealer_button == player
    }

    pub const fn is_current_dealer(&self) -> bool {
        self.is_dealer(self.current_player)
    }

    pub const fn is_finished(&self) -> bool {
        self.current_state == POKER_HAND_STATE_FINISHED
    }

    pub const fn get_current_player(&self) -> usize {
        self.current_player
    }

    pub fn next_dealer(&mut self) {
        self.current_player = self.dealer_button;
    }

    pub fn next_player(&mut self) -> bool {
        self.current_player = (self.current_player + 1) % self.num_players;
        self.current_player == self.dealer_button
    }

    pub fn next_player_masked(&mut self, mask: &Vec<bool>, from_dealer: bool) -> bool {
        if from_dealer {
            self.next_dealer();
            if mask[self.current_player] {
                return false;
            }
        }
        let current_player = self.current_player;
        loop {
            self.next_player();
            if mask[self.current_player] {
                return false;
            }
            if current_player == self.current_player {
                return true;
            }
        }
    }

    pub fn next_round(&mut self) -> Result<bool, Vec<u8>> {
        let next_round = self.current_round + 1;

        if next_round > self.max_rounds {
            return Err(b"No next round - Hand has finished")?;
        }

        self.current_round = next_round;

        if next_round == self.max_rounds {
            return Ok(true);
        }

        Ok(false)
    }

    pub const fn to_tuple(&self) -> (usize, usize, u8) {
        (self.current_round, self.current_player, self.current_state)
    }

    pub const fn to_enum(&self) -> PokerHandStateEnum {
        match self.current_state {
            POKER_HAND_STATE_SHUFFLE => PokerHandStateEnum::Shuffle {
                player: self.current_player,
                is_dealer: self.is_current_dealer(),
            },
            POKER_HAND_STATE_SMALL_BLIND => PokerHandStateEnum::SmallBlind {
                player: self.current_player,
            },
            POKER_HAND_STATE_BIG_BLIND => PokerHandStateEnum::BigBlind {
                player: self.current_player,
            },
            POKER_HAND_STATE_BET => PokerHandStateEnum::Bet {
                round: self.current_round,
                player: self.current_player,
            },
            POKER_HAND_STATE_UNMASK_HOLE_CARDS => PokerHandStateEnum::UnmaskHoleCards {
                player: self.current_player,
            },
            POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS => PokerHandStateEnum::UnmaskCommunityCards {
                round: self.current_round,
                player: self.current_player,
            },
            POKER_HAND_STATE_UNMASK_SHOWDOWN => PokerHandStateEnum::UnmaskShowdown {
                player: self.current_player,
            },
            POKER_HAND_STATE_SUBMIT_PUBLIC_KEY => PokerHandStateEnum::SubmitPublicKey {
                player: self.current_player,
            },
            POKER_HAND_STATE_CHEATED => PokerHandStateEnum::Cheated {
                player: self.current_player,
            },
            POKER_HAND_STATE_FINISHED => PokerHandStateEnum::Finished,
            _ => PokerHandStateEnum::Invalid,
        }
    }
}
