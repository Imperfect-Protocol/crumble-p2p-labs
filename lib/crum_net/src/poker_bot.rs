//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{error::Error, fmt};

use bls12_381::Scalar;
use crum_bls::{types::SigningKey, util::make_public_key_from_signing_key, verify};
use crum_pkr::{
    poker_deck::PokerCard,
    poker_hand::PokerHand,
    poker_state::{POKER_HOLDEM_ROUNDS, PokerHandStateEnum},
    poker_table::PokerTable,
};
use ff::Field;
use itertools::Itertools;
use rand::{
    Rng, SeedableRng,
    distributions::{Uniform, WeightedIndex},
    rngs::StdRng,
};

use crate::{Player, PlayerEvent, PokerAction, PokerGameState, PokerPlayerRegistrar};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokerBotError(String);

impl fmt::Display for PokerBotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for PokerBotError {}

impl From<String> for PokerBotError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for PokerBotError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<Vec<u8>> for PokerBotError {
    fn from(value: Vec<u8>) -> Self {
        Self(
            String::from_utf8(value)
                .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned()),
        )
    }
}

pub struct PokerBot {
    player_index: Option<usize>,
    sk: SigningKey,
    rng: StdRng,
    shuffle_trace: Option<Vec<verify::ShuffleTrace>>,
}

impl PokerBot {
    pub fn new() -> Self {
        let mut seed = [0u8; 32];
        seed[0] = 1;
        let mut rng = StdRng::from_seed(seed);
        let sk = Scalar::random(&mut rng);
        Self {
            player_index: None,
            sk,
            rng,
            shuffle_trace: None,
        }
    }

    pub fn seat_as(&mut self, player_index: usize) {
        let mut seed = [0u8; 32];
        seed[0] = (player_index as u8) + 1;
        self.rng = StdRng::from_seed(seed);
        self.sk = Scalar::random(&mut self.rng);
        self.player_index = Some(player_index);
        self.shuffle_trace = None;
    }

    pub fn player_index(&self) -> Result<usize, PokerBotError> {
        self.player_index
            .ok_or_else(|| PokerBotError::from("bot has not been seated at a table"))
    }

    pub fn act<State>(
        &mut self,
        state: &mut State,
        _events: &[PlayerEvent],
    ) -> Result<(), PokerBotError>
    where
        State: PokerGameState<Error = PokerBotError>,
    {
        let (action, log_lines) = self.build_action(state.table())?;
        state.submit_action(action, log_lines)
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

    fn build_action(
        &mut self,
        poker_table: &PokerTable,
    ) -> Result<(PokerAction, Vec<String>), PokerBotError> {
        let Some(hand) = poker_table.get_current_hand() else {
            return Err(PokerBotError::from("No active hand to act upon"));
        };

        let poker_state = hand.get_current_state().to_enum();
        match poker_state {
            PokerHandStateEnum::Shuffle { player, is_dealer } => {
                self.ensure_my_turn(player)?;
                let mut cards = if is_dealer {
                    hand.get_poker_deck().masked_cards()
                } else {
                    hand.get_shuffled_deck().clone()
                };
                cards.mask(self.sk);
                self.shuffle_trace
                    .replace(cards.shuffle_traced(&mut self.rng));

                Ok((
                    PokerAction::SubmitShuffledDeck {
                        player,
                        deck: cards,
                    },
                    vec![format!(
                        "Shuffle on Player {} (is_dealer={is_dealer})",
                        player + 1
                    )],
                ))
            }
            PokerHandStateEnum::SmallBlind { player } => {
                self.ensure_my_turn(player)?;
                Ok((
                    PokerAction::SubmitSmallBlind { player },
                    vec![format!("Small Blind on Player {}", player + 1)],
                ))
            }
            PokerHandStateEnum::BigBlind { player } => {
                self.ensure_my_turn(player)?;
                Ok((
                    PokerAction::SubmitBigBlind { player },
                    vec![format!("Big Blind on Player {}", player + 1)],
                ))
            }
            PokerHandStateEnum::Bet { round: _, player } => {
                self.ensure_my_turn(player)?;
                let min_bet = hand.get_call_amount_required(player)?;
                let small_blind = hand.get_small_blind();
                let chips = hand.get_chips_remaining(player);
                let bet = if chips < min_bet {
                    0
                } else {
                    let weights = [1, 4, 8];
                    let dist = WeightedIndex::new(&weights)
                        .map_err(|_| PokerBotError::from("Failed to create weighted index"))?;
                    let action = self.rng.sample(dist);
                    match action {
                        0 => 0,
                        1 => min_bet,
                        _ => {
                            let start_unit = min_bet.div_ceil(small_blind);
                            let end_unit = chips / small_blind;
                            if start_unit <= end_unit {
                                let units = self
                                    .rng
                                    .sample(Uniform::new_inclusive(start_unit, end_unit.min(10)));
                                units * small_blind
                            } else {
                                min_bet
                            }
                        }
                    }
                };

                Ok((
                    PokerAction::SubmitBet {
                        player,
                        amount: bet,
                    },
                    vec![format!(
                        "Player {} ({}) Bet: ${bet}",
                        player + 1,
                        player_own_cards_str(player, hand, self.sk)
                    )],
                ))
            }
            PokerHandStateEnum::UnmaskHoleCards { player } => {
                self.ensure_my_turn(player)?;
                let mut cards = hand.get_player_cards().clone();
                for (index, cards) in cards.iter_mut().enumerate() {
                    if index != player {
                        cards.unmask(self.sk);
                    }
                }

                Ok((
                    PokerAction::SubmitPlayerCards { player, cards },
                    vec![format!("Unmask Hole Cards on Player {}", player + 1)],
                ))
            }
            PokerHandStateEnum::UnmaskCommunityCards { round, player } => {
                self.ensure_my_turn(player)?;
                let Some(mut cards) = hand.get_community_cards(round).cloned() else {
                    return Err(PokerBotError::from("No community cards for round"));
                };
                cards.unmask(self.sk);

                Ok((
                    PokerAction::SubmitCommunityCards {
                        player,
                        round,
                        cards,
                    },
                    vec![format!(
                        "Round {} Unmask Community Cards on Player {}",
                        round + 1,
                        player + 1
                    )],
                ))
            }
            PokerHandStateEnum::UnmaskShowdown { player } => {
                self.ensure_my_turn(player)?;
                let mut cards = hand.get_player_cards().clone();
                if cards.get_mut(player).map(|c| c.unmask(self.sk)).is_none() {
                    return Err(PokerBotError::from("Invalid player cards for showdown"));
                }

                Ok((
                    PokerAction::SubmitShowdownCards { player, cards },
                    vec![format!("Unmask Showdown on Player {}", player + 1)],
                ))
            }
            PokerHandStateEnum::SubmitPublicKey { player } => {
                self.ensure_my_turn(player)?;
                let public_key = make_public_key_from_signing_key(&self.sk);
                let Some(traces) = self.shuffle_trace.take() else {
                    return Err(PokerBotError::from("No shuffle trace"));
                };

                Ok((
                    PokerAction::SubmitPublicKey {
                        player,
                        public_key,
                        traces,
                    },
                    vec![format!("Submit Public Key on Player {}", player + 1)],
                ))
            }
            PokerHandStateEnum::Finished => Err(PokerBotError::from("Hand is finished")),
            PokerHandStateEnum::Cheated { player } => Err(PokerBotError::from(format!(
                "Cheated by Player {}",
                player + 1
            ))),
            PokerHandStateEnum::Invalid => Err(PokerBotError::from("Invalid poker state")),
        }
    }

    fn ensure_my_turn(&self, player: usize) -> Result<(), PokerBotError> {
        let player_index = self.player_index()?;
        if player == player_index {
            Ok(())
        } else {
            Err(PokerBotError::from(format!(
                "Player {} tried to act for Player {}",
                player_index + 1,
                player + 1
            )))
        }
    }
}

impl Default for PokerBot {
    fn default() -> Self {
        Self::new()
    }
}

impl<State> Player<State, PlayerEvent> for PokerBot
where
    State: PokerGameState<Error = PokerBotError>,
{
    type Error = PokerBotError;

    fn act(&mut self, state: &mut State, events: &[PlayerEvent]) -> Result<(), Self::Error> {
        PokerBot::act(self, state, events)
    }
}

pub const NUM_PLAYERS: usize = 2;
pub const INITIAL_CHIPS: u64 = 1000;
pub const SMALL_BLIND: u64 = 10;
pub const TABLE_ID: u64 = 1;
pub const HAND_ID: u64 = 1;

pub fn new_table() -> Result<PokerTable, PokerBotError> {
    let mut poker_table = PokerTable::new(NUM_PLAYERS, POKER_HOLDEM_ROUNDS);
    for player_id in 1..=NUM_PLAYERS {
        poker_table.join(player_id as u32);
    }
    poker_table.start_hand(INITIAL_CHIPS, SMALL_BLIND)?;
    Ok(poker_table)
}

pub fn current_player(poker_table: &PokerTable) -> Result<usize, PokerBotError> {
    poker_table
        .get_current_hand()
        .map(|hand| hand.get_current_state().get_current_player())
        .ok_or_else(|| PokerBotError::from("Hand not started"))
}

pub fn is_finished(poker_table: &PokerTable) -> Result<bool, PokerBotError> {
    poker_table
        .get_current_hand()
        .map(|hand| hand.get_current_state().is_finished())
        .ok_or_else(|| PokerBotError::from("Hand not started"))
}

pub fn final_lines(poker_table: &PokerTable) -> Result<Vec<String>, PokerBotError> {
    let Some(hand) = poker_table.get_current_hand() else {
        return Err(PokerBotError::from("Hand not started"));
    };

    let mut lines = Vec::new();
    lines.extend(community_card_lines(hand));
    lines.extend(player_card_lines(hand));
    lines.push("Hand ended".to_string());
    Ok(lines)
}

struct PokerCards(Vec<Option<PokerCard>>);

impl ToString for PokerCards {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|opt_c| opt_c.as_ref().map_or("_!".to_string(), |c| c.to_string()))
            .join(", ")
    }
}

fn community_card_lines(hand: &PokerHand) -> Vec<String> {
    let mut community_cards = Vec::new();
    for round in 0..POKER_HOLDEM_ROUNDS {
        if let Some(cards) = hand.get_community_cards(round) {
            let cards = hand.get_poker_deck().unmasked_cards(cards);
            community_cards.extend(cards);
        }
    }
    vec![format!(
        "Community cards: {}",
        PokerCards(community_cards).to_string()
    )]
}

fn player_card_lines(hand: &PokerHand) -> Vec<String> {
    hand.get_player_cards()
        .iter()
        .enumerate()
        .map(|(index, cards)| {
            let cards = hand.get_poker_deck().unmasked_cards(cards);
            format!(
                "Player {} cards: {}",
                index + 1,
                PokerCards(cards).to_string()
            )
        })
        .collect()
}

fn player_own_cards_str(player: usize, hand: &PokerHand, sk: SigningKey) -> String {
    let cards = hand.get_player_cards();
    let mut cards = cards[player].clone();
    cards.unmask(sk);

    let cards = hand.get_poker_deck().unmasked_cards(&cards);
    PokerCards(cards).to_string()
}
