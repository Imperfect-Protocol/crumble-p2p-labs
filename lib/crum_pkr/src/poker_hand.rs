//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crum_bls::{types::PublicKey, verify};

use crate::{
    poker_bets::PokerBettingState,
    poker_deck::{MaskedCards, PokerDeck, UnmaskedCards},
    poker_state::{
        POKER_HAND_STATE_BET, POKER_HAND_STATE_BIG_BLIND, POKER_HAND_STATE_CHEATED,
        POKER_HAND_STATE_FINISHED, POKER_HAND_STATE_SMALL_BLIND,
        POKER_HAND_STATE_SUBMIT_PUBLIC_KEY, POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS,
        POKER_HAND_STATE_UNMASK_HOLE_CARDS, POKER_HAND_STATE_UNMASK_SHOWDOWN, POKER_HOLDEM_PREFLOP,
        PokerHandState, PokerHandStateEnum,
    },
};

pub struct PokerHand {
    /// player_keys[public keys]
    pub(super) poker_deck: PokerDeck,
    pub(super) shuffled_deck: MaskedCards,
    pub(super) shuffle_history: Vec<MaskedCards>,
    pub(super) player_cards: Vec<UnmaskedCards>,
    pub(super) player_keys: Vec<Option<PublicKey>>,
    pub(super) community_cards: Vec<UnmaskedCards>,
    pub(super) unmasking_sequence: Vec<(usize, u8, Vec<UnmaskedCards>)>,
    pub(super) current_state: PokerHandState,
    pub(super) betting_state: PokerBettingState,
    pub(super) small_blind: u64,
}

impl PokerHand {
    pub fn new(
        num_players: usize,
        max_rounds: usize,
        dealer_button: usize,
        initial_chips: u64,
        small_blind: u64,
    ) -> Self {
        let poker_deck = PokerDeck::new();
        let shuffled_deck = poker_deck.masked_cards();
        Self {
            poker_deck,
            shuffled_deck,
            shuffle_history: vec![],
            player_cards: (0..num_players).map(|_| UnmaskedCards::default()).collect(),
            player_keys: (0..num_players).map(|_| None).collect(),
            community_cards: (0..max_rounds).map(|_| UnmaskedCards::default()).collect(),
            unmasking_sequence: vec![],
            current_state: PokerHandState::new(num_players, max_rounds, dealer_button),
            betting_state: PokerBettingState::new(num_players, initial_chips),
            small_blind,
        }
    }

    /// On event acting player checks the current round to follow the rules
    /// Note: the Poker rounds are split into smaller rounds such as:
    /// Player 1 shuffles and submits, Player 2 shuffles submits, Player 1 blinds,
    /// Player 2 blids, Player 1 deals and unmasks for others, Player 2 unmasks for others
    /// Player 1 unmasks own, Player 2 unmasks own, Player 1 bets, Player 2 bets,
    /// Player 1 deals flop community cards and unmasks, Player 2 unmasks community cards,
    /// Player 1 bets, Player 2 bets, ... (and so on)
    pub const fn get_current_state(&self) -> &PokerHandState {
        &self.current_state
    }

    /// Poker deck is constant, but we ensure all players have same reference point
    pub const fn get_poker_deck(&self) -> &PokerDeck {
        &self.poker_deck
    }

    /// Supports Player shuffle and submit
    pub const fn get_shuffled_deck(&self) -> &MaskedCards {
        &self.shuffled_deck
    }

    /// Supports Player cards unmask
    pub fn get_player_cards(&self) -> &Vec<UnmaskedCards> {
        &self.player_cards
    }

    /// Supports community cards unmask
    pub fn get_community_cards(&self, round: usize) -> Option<&UnmaskedCards> {
        if round == POKER_HOLDEM_PREFLOP {
            return None;
        }
        self.community_cards.get(round - 1)
    }

    /// Tell amount required to call (minimum bet)
    pub fn get_call_amount_required(&self, player: usize) -> Result<u64, Vec<u8>> {
        self.betting_state.call_amount_required(player)
    }

    /// Tell amount of chips remaining
    pub fn get_chips_remaining(&self, player: usize) -> u64 {
        self.betting_state.chips_remaining(player)
    }

    /// Tell small blind amount
    pub fn get_small_blind(&self) -> u64 {
        self.small_blind
    }

    /// Tell big blind amount
    pub fn get_big_blind(&self) -> u64 {
        self.small_blind * 2
    }

    /// Called by each player to submit shuffled and masked deck
    pub fn submit_shuffled_deck(
        &mut self,
        player: usize,
        deck: MaskedCards,
    ) -> Result<(), Vec<u8>> {
        // check current player is submitter

        let PokerHandStateEnum::Shuffle {
            player: p,
            is_dealer: _,
        } = self.get_current_state().to_enum()
        else {
            return Err(b"Not in shuffle state")?;
        };

        if p != player {
            return Err(b"Not your turn to shuffle")?;
        }

        self.shuffle_history.push(deck.clone());
        self.shuffled_deck = deck;

        // emit shuffle submitted

        if self.current_state.next_player() {
            self.current_state.current_state = POKER_HAND_STATE_SMALL_BLIND;
        }

        Ok(())
    }

    pub fn submit_small_blind(&mut self, player: usize) -> Result<(), Vec<u8>> {
        let PokerHandStateEnum::SmallBlind { player: p } = self.get_current_state().to_enum()
        else {
            return Err(b"Not in small blind state")?;
        };

        if p != player {
            return Err(b"Not your turn to post small blind")?;
        }

        self.betting_state
            .process_action(player, self.get_small_blind())?;

        self.current_state.next_player();
        self.current_state.current_state = POKER_HAND_STATE_BIG_BLIND;

        Ok(())
    }

    pub fn submit_big_blind(&mut self, player: usize) -> Result<(), Vec<u8>> {
        let PokerHandStateEnum::BigBlind { player: p } = self.get_current_state().to_enum() else {
            return Err(b"Not in big blind state")?;
        };

        if p != player {
            return Err(b"Not your turn to post big blind")?;
        }

        self.betting_state
            .process_action(player, self.get_big_blind())?;

        for cards in self.player_cards.iter_mut() {
            *cards = self.shuffled_deck.deal(2);
        }

        self.current_state.next_dealer();
        self.current_state.current_state = POKER_HAND_STATE_UNMASK_HOLE_CARDS;

        Ok(())
    }

    /// Called by each player to unmask player hand
    pub fn submit_player_cards(
        &mut self,
        player: usize,
        player_cards: Vec<UnmaskedCards>,
    ) -> Result<bool, Vec<u8>> {
        // check current player is submitter
        let PokerHandStateEnum::UnmaskHoleCards { player: p } = self.get_current_state().to_enum()
        else {
            return Err(b"Not in unmask hole cards state")?;
        };

        if p != player {
            return Err(b"Not your turn to unmask hole cards")?;
        }

        if player_cards.len() != self.player_cards.len() {
            return Err(b"Incorrect length of player cards")?;
        }

        self.unmasking_sequence.push((
            player,
            POKER_HAND_STATE_UNMASK_HOLE_CARDS,
            player_cards.clone(),
        ));
        self.player_cards = player_cards;

        // emit player cards unmasked by player

        if self.current_state.next_player() {
            self.current_state
                .next_player_masked(self.betting_state.get_active_players(), true);
            self.betting_state.next_street();
            self.current_state.current_state = POKER_HAND_STATE_BET;

            self.check_betting_round_complete()?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Called by each player to unmask player hand
    pub fn submit_player_cards_showdown(
        &mut self,
        player: usize,
        player_cards: Vec<UnmaskedCards>,
    ) -> Result<bool, Vec<u8>> {
        // check current player is submitter
        let PokerHandStateEnum::UnmaskShowdown { player: p } = self.get_current_state().to_enum()
        else {
            return Err(b"Not in unmask hole cards state")?;
        };

        if p != player {
            return Err(b"Not your turn to unmask hole cards")?;
        }

        if player_cards.len() != self.player_cards.len() {
            return Err(b"Incorrect length of player cards")?;
        }

        self.unmasking_sequence.push((
            player,
            POKER_HAND_STATE_UNMASK_SHOWDOWN,
            player_cards.clone(),
        ));
        self.player_cards = player_cards;

        // emit player cards unmasked by player

        if self.current_state.next_player() {
            self.current_state.current_state = POKER_HAND_STATE_SUBMIT_PUBLIC_KEY;
            return Ok(true);
        }

        Ok(false)
    }

    /// Called by each player to unmask community cards
    pub fn submit_community_cards(
        &mut self,
        player: usize,
        round: usize,
        cards: UnmaskedCards,
    ) -> Result<bool, Vec<u8>> {
        // check current player is submitter
        let PokerHandStateEnum::UnmaskCommunityCards {
            round: r,
            player: p,
        } = self.get_current_state().to_enum()
        else {
            return Err(b"Not in bet state")?;
        };

        if r != round {
            return Err(b"Not this round to unmask cards")?;
        }

        if p != player {
            return Err(b"Not your turn to bet")?;
        }

        let round_cards = self
            .community_cards
            .get_mut(round - 1)
            .expect("No round cards");

        self.unmasking_sequence.push((
            player,
            POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS,
            vec![cards.clone()],
        ));
        *round_cards = cards;

        // emit community cards for round unmasked by player

        if self.current_state.next_player() {
            self.current_state
                .next_player_masked(self.betting_state.get_active_players(), true);
            self.betting_state.next_street();
            self.current_state.current_state = POKER_HAND_STATE_BET;

            self.check_betting_round_complete()?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Called at the end of hand to verify faierness of gameplay
    pub fn submit_public_key(
        &mut self,
        player: usize,
        pk: PublicKey,
        traces: Vec<verify::ShuffleTrace>,
    ) -> Result<(), Vec<u8>> {
        let PokerHandStateEnum::SubmitPublicKey { player: p } = self.get_current_state().to_enum()
        else {
            return Err(b"Not in submit public key state")?;
        };

        if p != player {
            return Err(b"Not your turn to submit public key")?;
        }

        let player_key = self.player_keys.get_mut(player).expect("No player key");
        *player_key = Some(pk);

        // emit (ephemeral) public key submitted

        if !self.verify_shuffle(player, pk, traces) {
            self.current_state.current_state = POKER_HAND_STATE_CHEATED;
            return Err("Player cheated during shuffle")?;
        }

        if self.current_state.next_player() {
            match self.verify_unmasking() {
                Ok(None) => (),
                Ok(Some(cheater)) => {
                    self.current_state.current_state = POKER_HAND_STATE_CHEATED;
                    return Err(
                        format!("Player cheated during unmasking {}", cheater).into_bytes()
                    )?;
                }
                Err(err) => Err(err)?,
            }
            // TODO
            // compute score of each hand
            // select winner
            self.current_state.current_state = POKER_HAND_STATE_FINISHED;
        }

        Ok(())
    }

    pub fn verify_shuffle(
        &mut self,
        player: usize,
        pk: PublicKey,
        traces: Vec<verify::ShuffleTrace>,
    ) -> bool {
        let num_players = self.current_state.num_players;
        let dealer = self.current_state.dealer_button;

        let step_index = (player + num_players - dealer) % num_players;

        let next_cards = self.shuffle_history[step_index].cards();
        let prev_cards = if step_index == 0 {
            self.poker_deck.cards()
        } else {
            self.shuffle_history[step_index - 1].cards()
        };

        verify::verify_shuffle_traced(&prev_cards, &next_cards, &pk, &traces).is_ok()
    }

    pub fn submit_bet(&mut self, player: usize, amount: u64) -> Result<(), Vec<u8>> {
        let PokerHandStateEnum::Bet {
            round: _,
            player: p,
        } = self.get_current_state().to_enum()
        else {
            return Err(b"Not in bet state")?;
        };

        if p != player {
            return Err(b"Not your turn to bet")?;
        }

        self.betting_state.process_action(player, amount)?;
        self.current_state
            .next_player_masked(self.betting_state.get_active_players(), false);

        self.check_betting_round_complete()?;

        Ok(())
    }

    fn check_betting_round_complete(&mut self) -> Result<(), Vec<u8>> {
        if self.betting_state.is_betting_round_complete() {
            self.current_state.next_dealer();
            let round = self.current_state.current_round;

            if self.current_state.next_round()? {
                self.current_state.current_state = POKER_HAND_STATE_UNMASK_SHOWDOWN;
            } else {
                let num_cards_deal = if round == POKER_HOLDEM_PREFLOP { 3 } else { 1 };
                self.community_cards[round] = self.shuffled_deck.deal(num_cards_deal);
                self.current_state.current_state = POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS;
            }
        }
        Ok(())
    }
}
