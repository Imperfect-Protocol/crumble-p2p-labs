//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{error::Error, fmt};

use bls12_381::G1Affine;

use crum_pkr::poker_deck::{MaskedCards, UnmaskedCards};

pub const G1AFFINE_COMPRESSED_LEN: usize = 48;

pub type ContractCardBytes = Vec<u8>;
pub type ContractCardsBytes = Vec<ContractCardBytes>;
pub type ContractCardSetsBytes = Vec<ContractCardsBytes>;
pub type ContractUnmaskingSequenceBytes = Vec<ContractCardSetsBytes>;

pub fn serialize_g1_affine(point: &G1Affine) -> ContractCardBytes {
    point.to_compressed().to_vec()
}

pub fn deserialize_g1_affine(bytes: &[u8]) -> Result<G1Affine, PokerDeckSerializationError> {
    if bytes.len() != G1AFFINE_COMPRESSED_LEN {
        return Err(PokerDeckSerializationError::InvalidG1CompressedLength {
            expected: G1AFFINE_COMPRESSED_LEN,
            actual: bytes.len(),
        });
    }

    let mut compressed = [0u8; G1AFFINE_COMPRESSED_LEN];
    compressed.copy_from_slice(bytes);

    G1Affine::from_compressed(&compressed)
        .into_option()
        .ok_or(PokerDeckSerializationError::G1DecodeError)
}

pub fn serialize_masked_cards(cards: &MaskedCards) -> ContractCardsBytes {
    serialize_g1_cards(cards.cards())
}

pub fn deserialize_masked_cards<T>(cards: &[T]) -> Result<MaskedCards, PokerDeckSerializationError>
where
    T: AsRef<[u8]>,
{
    deserialize_g1_cards(cards).map(MaskedCards::new)
}

pub fn serialize_masked_card_sets(card_sets: &[MaskedCards]) -> ContractCardSetsBytes {
    card_sets.iter().map(serialize_masked_cards).collect()
}

pub fn deserialize_masked_card_sets<T, U>(
    card_sets: &[T],
) -> Result<Vec<MaskedCards>, PokerDeckSerializationError>
where
    T: AsRef<[U]>,
    U: AsRef<[u8]>,
{
    card_sets
        .iter()
        .map(|cards| deserialize_masked_cards(cards.as_ref()))
        .collect()
}

pub fn serialize_unmasked_cards(cards: &UnmaskedCards) -> ContractCardsBytes {
    serialize_g1_cards(cards.cards())
}

pub fn deserialize_unmasked_cards<T>(
    cards: &[T],
) -> Result<UnmaskedCards, PokerDeckSerializationError>
where
    T: AsRef<[u8]>,
{
    deserialize_g1_cards(cards).map(UnmaskedCards::new)
}

pub fn serialize_unmasked_card_sets(card_sets: &[UnmaskedCards]) -> Vec<ContractCardsBytes> {
    card_sets.iter().map(serialize_unmasked_cards).collect()
}

pub fn deserialize_unmasked_card_sets<T, U>(
    card_sets: &[T],
) -> Result<Vec<UnmaskedCards>, PokerDeckSerializationError>
where
    T: AsRef<[U]>,
    U: AsRef<[u8]>,
{
    card_sets
        .iter()
        .map(|cards| deserialize_unmasked_cards(cards.as_ref()))
        .collect()
}

pub fn serialize_unmasking_sequence(
    sequence: &[Vec<UnmaskedCards>],
) -> ContractUnmaskingSequenceBytes {
    sequence
        .iter()
        .map(|stage| serialize_unmasked_card_sets(stage))
        .collect()
}

pub fn deserialize_unmasking_sequence<T, U, V>(
    sequence: &[T],
) -> Result<Vec<Vec<UnmaskedCards>>, PokerDeckSerializationError>
where
    T: AsRef<[U]>,
    U: AsRef<[V]>,
    V: AsRef<[u8]>,
{
    sequence
        .iter()
        .map(|stage| deserialize_unmasked_card_sets(stage.as_ref()))
        .collect()
}

fn serialize_g1_cards(cards: Vec<G1Affine>) -> ContractCardsBytes {
    cards
        .iter()
        .map(serialize_g1_affine)
        .collect::<ContractCardsBytes>()
}

fn deserialize_g1_cards<T>(cards: &[T]) -> Result<Vec<G1Affine>, PokerDeckSerializationError>
where
    T: AsRef<[u8]>,
{
    cards
        .iter()
        .map(|card| deserialize_g1_affine(card.as_ref()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PokerDeckSerializationError {
    InvalidG1CompressedLength { expected: usize, actual: usize },
    G1DecodeError,
}

impl fmt::Display for PokerDeckSerializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidG1CompressedLength { expected, actual } => write!(
                formatter,
                "invalid compressed G1 length: expected {expected}, received {actual}"
            ),
            Self::G1DecodeError => write!(formatter, "could not decode compressed G1 point"),
        }
    }
}

impl Error for PokerDeckSerializationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crum_pkr::poker_deck::PokerDeck;

    #[test]
    fn masked_cards_round_trip_as_contract_bytes() {
        let deck = PokerDeck::new();
        let cards = deck.masked_cards();

        let encoded = serialize_masked_cards(&cards);
        assert_eq!(encoded.len(), deck.len());
        assert!(encoded.iter().all(|card| card.len() == 48));

        let decoded = deserialize_masked_cards(&encoded).expect("decode masked cards");
        assert_eq!(decoded.cards(), cards.cards());
    }

    #[test]
    fn unmasked_cards_round_trip_as_contract_bytes() {
        let mut deck = PokerDeck::new().masked_cards();
        let cards = deck.deal(2);

        let encoded = serialize_unmasked_cards(&cards);
        assert_eq!(encoded.len(), 2);
        assert!(encoded.iter().all(|card| card.len() == 48));

        let decoded = deserialize_unmasked_cards(&encoded).expect("decode unmasked cards");
        assert_eq!(decoded.cards(), cards.cards());
    }

    #[test]
    fn nested_unmasked_card_sets_round_trip() {
        let mut deck = PokerDeck::new().masked_cards();
        let first = deck.deal(2);
        let second = deck.deal(2);
        let sets = vec![first, second];

        let encoded = serialize_unmasked_card_sets(&sets);
        let decoded = deserialize_unmasked_card_sets(&encoded).expect("decode card sets");

        assert_eq!(
            decoded.iter().map(UnmaskedCards::cards).collect::<Vec<_>>(),
            sets.iter().map(UnmaskedCards::cards).collect::<Vec<_>>()
        );
    }

    #[test]
    fn nested_masked_card_sets_round_trip() {
        let deck = PokerDeck::new();
        let sets = vec![deck.masked_cards(), deck.masked_cards()];

        let encoded = serialize_masked_card_sets(&sets);
        let decoded = deserialize_masked_card_sets(&encoded).expect("decode masked card sets");

        assert_eq!(
            decoded.iter().map(MaskedCards::cards).collect::<Vec<_>>(),
            sets.iter().map(MaskedCards::cards).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unmasking_sequence_round_trip() {
        let mut deck = PokerDeck::new().masked_cards();
        let sequence = vec![vec![deck.deal(2), deck.deal(2)], vec![deck.deal(3)]];

        let encoded = serialize_unmasking_sequence(&sequence);
        let decoded = deserialize_unmasking_sequence(&encoded).expect("decode sequence");

        assert_eq!(
            decoded
                .iter()
                .map(|stage| stage.iter().map(UnmaskedCards::cards).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            sequence
                .iter()
                .map(|stage| stage.iter().map(UnmaskedCards::cards).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rejects_wrong_g1_length() {
        let err = deserialize_g1_affine(&[0u8; 47]).expect_err("wrong length");

        assert_eq!(
            err,
            PokerDeckSerializationError::InvalidG1CompressedLength {
                expected: 48,
                actual: 47,
            }
        );
    }

    #[test]
    fn rejects_invalid_g1_encoding() {
        let err = deserialize_g1_affine(&[0u8; 48]).expect_err("invalid encoding");

        assert_eq!(err, PokerDeckSerializationError::G1DecodeError);
    }
}
