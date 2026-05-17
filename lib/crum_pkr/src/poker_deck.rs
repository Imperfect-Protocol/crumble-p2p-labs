//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use alloy_primitives::Keccak256;
use bls12_381::G1Affine;
use crum_bls::{hash_to_curve::hash_to_curve, sign, types::SigningKey, verify};
use pairing::group::Curve;
use rand::{Rng, seq::SliceRandom};

#[derive(Default, Clone, Debug)]
pub struct PokerCard(Vec<u8>);

impl ToString for PokerCard {
    fn to_string(&self) -> String {
        String::from_utf8(self.0.clone()).unwrap()
    }
}

#[derive(Default, Clone, Debug)]
pub struct PokerDeck {
    poker_cards: Vec<PokerCard>,
    cards_g1: Vec<G1Affine>,
}

impl PokerDeck {
    pub fn new() -> Self {
        let poker_cards: Vec<PokerCard> = b"23456789TJQKA"
            .iter()
            .flat_map(|rank| b"shdc".iter().map(move |suit| vec![*rank, *suit]))
            .map(|v| PokerCard(v))
            .collect();

        let cards_g1: Vec<G1Affine> = poker_cards
            .iter()
            .map(|card| hash_to_curve(&card.0).to_affine())
            .collect();

        Self {
            poker_cards,
            cards_g1,
        }
    }

    pub fn find_card(&self, revealed_point: G1Affine) -> Option<PokerCard> {
        let Some(card_index) = self.cards_g1.iter().position(|x| revealed_point.eq(x)) else {
            return None;
        };

        self.poker_cards.get(card_index).cloned()
    }

    pub fn cards(&self) -> Vec<G1Affine> {
        self.cards_g1.clone()
    }

    pub fn len(&self) -> usize {
        self.cards_g1.len()
    }

    pub fn masked_cards(&self) -> MaskedCards {
        MaskedCards::new(self.cards())
    }

    pub fn unmasked_cards(&self, unmasked_cards: &UnmaskedCards) -> Vec<Option<PokerCard>> {
        unmasked_cards
            .cards_g1
            .iter()
            .map(|card_g1| self.find_card(*card_g1))
            .collect()
    }
}

#[derive(Default, Clone, Debug)]
pub struct MaskedCards {
    cards_g1: Vec<G1Affine>,
}

impl MaskedCards {
    pub fn new(cards_g1: Vec<G1Affine>) -> Self {
        Self { cards_g1 }
    }

    pub fn cards(&self) -> Vec<G1Affine> {
        self.cards_g1.clone()
    }

    pub fn cards_n(&self, count: usize) -> Vec<G1Affine> {
        self.cards_g1[0..count].iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.cards_g1.len()
    }

    pub fn mask(&mut self, sk: SigningKey) {
        self.cards_g1
            .iter_mut()
            .for_each(|card_g1| *card_g1 = sign::mask(*card_g1, sk));
    }

    pub fn shuffle(&mut self, rng: &mut impl Rng) {
        self.cards_g1.shuffle(rng);
    }

    pub fn shuffle_traced(&mut self, rng: &mut impl Rng) -> Vec<verify::ShuffleTrace> {
        let mut cards_g1_indexed: Vec<_> = self.cards_g1.iter().cloned().enumerate().collect();

        cards_g1_indexed.shuffle(rng);

        let res = cards_g1_indexed
            .iter()
            .enumerate()
            .map(|(after_index, (before_index, _))| verify::ShuffleTrace {
                after_index,
                claimed_before_index: *before_index,
            })
            .collect();

        self.cards_g1 = cards_g1_indexed.into_iter().map(|(_, c)| c).collect();
        res
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        for card in &self.cards_g1 {
            hasher.update(&card.to_compressed());
        }
        hasher.finalize().into()
    }

    pub fn deal(&mut self, count: usize) -> UnmaskedCards {
        let dealt_cards = self.cards_g1.drain(..count).collect();
        UnmaskedCards::new(dealt_cards)
    }
}

#[derive(Default, Clone, Debug)]
pub struct UnmaskedCards {
    cards_g1: Vec<G1Affine>,
}

impl UnmaskedCards {
    pub fn new(cards_g1: Vec<G1Affine>) -> Self {
        Self { cards_g1 }
    }

    pub fn cards(&self) -> Vec<G1Affine> {
        self.cards_g1.clone()
    }

    pub fn len(&self) -> usize {
        self.cards_g1.len()
    }

    pub fn unmask(&mut self, sk: SigningKey) {
        let sk_inv = sk.invert().expect("Invalid signing key");
        self.cards_g1
            .iter_mut()
            .for_each(|card_g1| *card_g1 = sign::mask(*card_g1, sk_inv));
    }
}
