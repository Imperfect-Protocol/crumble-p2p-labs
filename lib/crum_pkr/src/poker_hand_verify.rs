//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use super::poker_hand::PokerHand;
use pairing::{MultiMillerLoop, group::Group};

use crate::poker_state::{
    POKER_HAND_STATE_CHEATED, POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS,
    POKER_HAND_STATE_UNMASK_HOLE_CARDS, POKER_HAND_STATE_UNMASK_SHOWDOWN,
};

impl PokerHand {
    /// Replay and verify whole unmasking history.
    ///
    /// This is efficient algorithm using only single Final Exponentiation call.
    ///
    pub fn verify_unmasking(&mut self) -> Result<Option<usize>, Vec<u8>> {
        let final_shuffled_deck = self
            .shuffle_history
            .last()
            .ok_or_else(|| b"No shuffle history")?
            .cards();

        let num_players = self.current_state.num_players;
        let mut deck_idx = 0;

        let mut tracked_hole_cards: Vec<Vec<bls12_381::G1Affine>> = Vec::new();
        for _ in 0..num_players {
            tracked_hole_cards.push(final_shuffled_deck[deck_idx..deck_idx + 2].to_vec());
            deck_idx += 2;
        }

        let mut tracked_community_cards: Vec<Vec<bls12_381::G1Affine>> = vec![
            final_shuffled_deck[deck_idx..deck_idx + 3].to_vec(),
            final_shuffled_deck[deck_idx + 3..deck_idx + 4].to_vec(),
            final_shuffled_deck[deck_idx + 4..deck_idx + 5].to_vec(),
        ];

        let mut comm_round_idx = 0;
        let mut comm_unmask_count = 0;

        // 1. Prepare G2 points once for the entire batch to save CPU cycles
        let neg_g2_gen = -bls12_381::G2Affine::generator();
        let neg_g2_prepared = bls12_381::G2Prepared::from(neg_g2_gen);

        let mut prepared_pks = Vec::new();
        for pk_opt in &self.player_keys {
            let pk = pk_opt.ok_or_else(|| b"Missing PK for unmask audit")?;
            prepared_pks.push(bls12_381::G2Prepared::from(pk));
        }

        // We will collect all peeling actions here: (unmasked, masked, action_player)
        let mut audit_trail = Vec::new();

        // 2. Replay history and collect the trace instead of verifying immediately
        for (action_player, state_type, submitted_cards) in &self.unmasking_sequence {
            match *state_type {
                POKER_HAND_STATE_UNMASK_HOLE_CARDS => {
                    for target_player in 0..num_players {
                        if target_player == *action_player {
                            continue;
                        }
                        let before = &tracked_hole_cards[target_player];
                        let after = submitted_cards[target_player].cards();

                        for (b, a) in before.iter().zip(after.iter()) {
                            audit_trail.push((*a, *b, *action_player));
                        }
                        tracked_hole_cards[target_player] = after;
                    }
                }
                POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS => {
                    let before = &tracked_community_cards[comm_round_idx];
                    let after = submitted_cards[0].cards();

                    for (b, a) in before.iter().zip(after.iter()) {
                        audit_trail.push((*a, *b, *action_player));
                    }
                    tracked_community_cards[comm_round_idx] = after;

                    comm_unmask_count += 1;
                    if comm_unmask_count == num_players {
                        comm_unmask_count = 0;
                        comm_round_idx += 1;
                    }
                }
                POKER_HAND_STATE_UNMASK_SHOWDOWN => {
                    let target_player = *action_player;
                    let before = &tracked_hole_cards[target_player];
                    let after = submitted_cards[target_player].cards();

                    for (b, a) in before.iter().zip(after.iter()) {
                        audit_trail.push((*a, *b, *action_player));
                    }
                    tracked_hole_cards[target_player] = after;
                }
                _ => {}
            }
        }

        // 3. Build the giant batch for the Miller Loop
        let mut miller_terms = Vec::with_capacity(audit_trail.len() * 2);
        for (unmasked, masked, action_player) in &audit_trail {
            miller_terms.push((unmasked, &prepared_pks[*action_player]));
            miller_terms.push((masked, &neg_g2_prepared));
        }

        // 4. The Optimistic Batch Execution (O(1) final exponentiation for the whole game)
        let is_valid: bool = bls12_381::Bls12::multi_miller_loop(&miller_terms)
            .final_exponentiation()
            .is_identity()
            .into();

        if is_valid {
            // The game was perfectly fair.
            return Ok(None);
        }

        // 5. Fallback: The batch failed. Someone cheated.
        // We run the individual checks to find out exactly who it was.
        for (unmasked, masked, action_player) in audit_trail {
            let is_match: bool = bls12_381::Bls12::multi_miller_loop(&[
                (&unmasked, &prepared_pks[action_player]),
                (&masked, &neg_g2_prepared),
            ])
            .final_exponentiation()
            .is_identity()
            .into();

            if !is_match {
                self.current_state.current_state = POKER_HAND_STATE_CHEATED;
                return Ok(Some(action_player));
            }
        }

        Ok(None)
    }
}
