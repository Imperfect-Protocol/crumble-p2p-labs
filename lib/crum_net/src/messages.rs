//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::collections::HashSet;

use crum_bls::{
    types::PublicKey,
    util::{PUBLIC_KEY_COMPRESSED_LEN, make_public_key_from_compressed_slice},
    verify::ShuffleTrace,
};
use crum_pkr::{
    poker_deck::{MaskedCards, UnmaskedCards},
    poker_hand::PokerHand,
    poker_table::PokerTable,
};
use serde::{Deserialize, Serialize};

use crate::{
    TopologyPolicy,
    poker_deck_serialization::{
        ContractCardSetsBytes, ContractCardsBytes, deserialize_masked_cards,
        deserialize_unmasked_card_sets, deserialize_unmasked_cards, serialize_masked_cards,
        serialize_unmasked_card_sets, serialize_unmasked_cards,
    },
};

pub type PlayerIndex = usize;
pub type PeerId = String;
pub type HandId = u64;
pub type TableId = u64;
pub type Sequence = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerAnnouncement {
    pub peer_id: PeerId,
    pub player_index: PlayerIndex,
    pub table_id: TableId,
    pub annonce_public_key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointDescriptor {
    pub public_ip: String,
    pub public_port: u16,
    pub transport_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeSignal {
    pub table_id: TableId,
    pub hand_id: HandId,
    pub from: PeerId,
    pub to: PeerId,
    pub topology: TopologyPolicy,
    pub encrypted_endpoint: Vec<u8>,
    pub nonce: u64,
}

#[derive(Clone)]
pub struct PokerActionEnvelope {
    pub table_id: TableId,
    pub hand_id: HandId,
    pub from: PlayerIndex,
    pub sequence: Sequence,
    pub expected_state: (usize, usize, u8),
    pub action: PokerAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PokerActionEnvelopeWire {
    pub table_id: TableId,
    pub hand_id: HandId,
    pub from: PlayerIndex,
    pub sequence: Sequence,
    pub expected_state: (usize, usize, u8),
    pub action: PokerActionWire,
    pub log_lines: Vec<String>,
}

impl PokerActionEnvelope {
    pub fn to_wire(&self, log_lines: Vec<String>) -> PokerActionEnvelopeWire {
        PokerActionEnvelopeWire {
            table_id: self.table_id,
            hand_id: self.hand_id,
            from: self.from,
            sequence: self.sequence,
            expected_state: self.expected_state,
            action: self.action.to_wire(),
            log_lines,
        }
    }
}

impl PokerActionEnvelopeWire {
    pub fn to_envelope(&self) -> Result<PokerActionEnvelope, String> {
        Ok(PokerActionEnvelope {
            table_id: self.table_id,
            hand_id: self.hand_id,
            from: self.from,
            sequence: self.sequence,
            expected_state: self.expected_state,
            action: self.action.to_action()?,
        })
    }
}

#[derive(Clone)]
pub enum P2pMessage {
    Handshake(HandshakeSignal),
    PokerAction(PokerActionEnvelope),
    Ack {
        from: PeerId,
        sequence: Sequence,
    },
    StateDigest {
        from: PeerId,
        hand_id: HandId,
        digest: [u8; 32],
    },
}

pub enum PokerAction {
    SubmitShuffledDeck {
        player: PlayerIndex,
        deck: MaskedCards,
    },
    SubmitSmallBlind {
        player: PlayerIndex,
    },
    SubmitBigBlind {
        player: PlayerIndex,
    },
    SubmitPlayerCards {
        player: PlayerIndex,
        cards: Vec<UnmaskedCards>,
    },
    SubmitCommunityCards {
        player: PlayerIndex,
        round: usize,
        cards: UnmaskedCards,
    },
    SubmitShowdownCards {
        player: PlayerIndex,
        cards: Vec<UnmaskedCards>,
    },
    SubmitBet {
        player: PlayerIndex,
        amount: u64,
    },
    SubmitPublicKey {
        player: PlayerIndex,
        public_key: PublicKey,
        traces: Vec<ShuffleTrace>,
    },
}

impl Clone for PokerAction {
    fn clone(&self) -> Self {
        match self {
            Self::SubmitShuffledDeck { player, deck } => Self::SubmitShuffledDeck {
                player: *player,
                deck: deck.clone(),
            },
            Self::SubmitSmallBlind { player } => Self::SubmitSmallBlind { player: *player },
            Self::SubmitBigBlind { player } => Self::SubmitBigBlind { player: *player },
            Self::SubmitPlayerCards { player, cards } => Self::SubmitPlayerCards {
                player: *player,
                cards: cards.clone(),
            },
            Self::SubmitCommunityCards {
                player,
                round,
                cards,
            } => Self::SubmitCommunityCards {
                player: *player,
                round: *round,
                cards: cards.clone(),
            },
            Self::SubmitShowdownCards { player, cards } => Self::SubmitShowdownCards {
                player: *player,
                cards: cards.clone(),
            },
            Self::SubmitBet { player, amount } => Self::SubmitBet {
                player: *player,
                amount: *amount,
            },
            Self::SubmitPublicKey {
                player,
                public_key,
                traces,
            } => Self::SubmitPublicKey {
                player: *player,
                public_key: *public_key,
                traces: traces
                    .iter()
                    .map(|trace| ShuffleTrace {
                        after_index: trace.after_index,
                        claimed_before_index: trace.claimed_before_index,
                    })
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PokerActionWire {
    SubmitShuffledDeck {
        player: PlayerIndex,
        deck_g1_compressed: ContractCardsBytes,
    },
    SubmitSmallBlind {
        player: PlayerIndex,
    },
    SubmitBigBlind {
        player: PlayerIndex,
    },
    SubmitPlayerCards {
        player: PlayerIndex,
        player_cards_g1_compressed: ContractCardSetsBytes,
    },
    SubmitCommunityCards {
        player: PlayerIndex,
        round: usize,
        cards_g1_compressed: ContractCardsBytes,
    },
    SubmitShowdownCards {
        player: PlayerIndex,
        player_cards_g1_compressed: ContractCardSetsBytes,
    },
    SubmitBet {
        player: PlayerIndex,
        amount: u64,
    },
    SubmitPublicKey {
        player: PlayerIndex,
        public_key_g2_compressed: Vec<u8>,
        traces: Vec<(usize, usize)>,
    },
}

impl PokerAction {
    pub fn to_wire(&self) -> PokerActionWire {
        match self {
            Self::SubmitShuffledDeck { player, deck } => PokerActionWire::SubmitShuffledDeck {
                player: *player,
                deck_g1_compressed: serialize_masked_cards(deck),
            },
            Self::SubmitSmallBlind { player } => {
                PokerActionWire::SubmitSmallBlind { player: *player }
            }
            Self::SubmitBigBlind { player } => PokerActionWire::SubmitBigBlind { player: *player },
            Self::SubmitPlayerCards { player, cards } => PokerActionWire::SubmitPlayerCards {
                player: *player,
                player_cards_g1_compressed: serialize_unmasked_card_sets(cards),
            },
            Self::SubmitCommunityCards {
                player,
                round,
                cards,
            } => PokerActionWire::SubmitCommunityCards {
                player: *player,
                round: *round,
                cards_g1_compressed: serialize_unmasked_cards(cards),
            },
            Self::SubmitShowdownCards { player, cards } => PokerActionWire::SubmitShowdownCards {
                player: *player,
                player_cards_g1_compressed: serialize_unmasked_card_sets(cards),
            },
            Self::SubmitBet { player, amount } => PokerActionWire::SubmitBet {
                player: *player,
                amount: *amount,
            },
            Self::SubmitPublicKey {
                player,
                public_key,
                traces,
            } => PokerActionWire::SubmitPublicKey {
                player: *player,
                public_key_g2_compressed: public_key.to_compressed().to_vec(),
                traces: traces
                    .iter()
                    .map(|trace| (trace.after_index, trace.claimed_before_index))
                    .collect(),
            },
        }
    }
}

impl PokerActionWire {
    pub fn to_action(&self) -> Result<PokerAction, String> {
        match self {
            Self::SubmitShuffledDeck {
                player,
                deck_g1_compressed,
            } => Ok(PokerAction::SubmitShuffledDeck {
                player: *player,
                deck: deserialize_masked_cards(deck_g1_compressed)
                    .map_err(|err| err.to_string())?,
            }),
            Self::SubmitSmallBlind { player } => {
                Ok(PokerAction::SubmitSmallBlind { player: *player })
            }
            Self::SubmitBigBlind { player } => Ok(PokerAction::SubmitBigBlind { player: *player }),
            Self::SubmitPlayerCards {
                player,
                player_cards_g1_compressed,
            } => Ok(PokerAction::SubmitPlayerCards {
                player: *player,
                cards: deserialize_unmasked_card_sets(player_cards_g1_compressed)
                    .map_err(|err| err.to_string())?,
            }),
            Self::SubmitCommunityCards {
                player,
                round,
                cards_g1_compressed,
            } => Ok(PokerAction::SubmitCommunityCards {
                player: *player,
                round: *round,
                cards: deserialize_unmasked_cards(cards_g1_compressed)
                    .map_err(|err| err.to_string())?,
            }),
            Self::SubmitShowdownCards {
                player,
                player_cards_g1_compressed,
            } => Ok(PokerAction::SubmitShowdownCards {
                player: *player,
                cards: deserialize_unmasked_card_sets(player_cards_g1_compressed)
                    .map_err(|err| err.to_string())?,
            }),
            Self::SubmitBet { player, amount } => Ok(PokerAction::SubmitBet {
                player: *player,
                amount: *amount,
            }),
            Self::SubmitPublicKey {
                player,
                public_key_g2_compressed,
                traces,
            } => {
                if public_key_g2_compressed.len() != PUBLIC_KEY_COMPRESSED_LEN {
                    return Err(format!(
                        "invalid compressed G2 public-key length: expected {}, received {}",
                        PUBLIC_KEY_COMPRESSED_LEN,
                        public_key_g2_compressed.len()
                    ));
                }
                Ok(PokerAction::SubmitPublicKey {
                    player: *player,
                    public_key: make_public_key_from_compressed_slice(public_key_g2_compressed)
                        .map_err(|err| err.to_string())?,
                    traces: traces
                        .iter()
                        .map(|(after_index, claimed_before_index)| ShuffleTrace {
                            after_index: *after_index,
                            claimed_before_index: *claimed_before_index,
                        })
                        .collect(),
                })
            }
        }
    }
}

pub struct PokerClientState {
    pub player_index: PlayerIndex,
    pub table_id: TableId,
    pub hand_id: HandId,
    sequence: Sequence,
    seen: HashSet<(PlayerIndex, Sequence)>,
    pub table: PokerTable,
}

impl PokerClientState {
    pub fn new(
        player_index: PlayerIndex,
        table_id: TableId,
        hand_id: HandId,
        table: PokerTable,
    ) -> Self {
        Self {
            player_index,
            table_id,
            hand_id,
            sequence: 0,
            seen: HashSet::new(),
            table,
        }
    }

    pub fn submit_local_action(
        &mut self,
        action: PokerAction,
    ) -> Result<PokerActionEnvelope, Vec<u8>> {
        let expected_state = self.current_state_tuple()?;
        self.apply_action(action.clone())?;

        self.sequence += 1;
        self.seen.insert((self.player_index, self.sequence));
        Ok(PokerActionEnvelope {
            table_id: self.table_id,
            hand_id: self.hand_id,
            from: self.player_index,
            sequence: self.sequence,
            expected_state,
            action,
        })
    }

    pub fn apply_envelope(&mut self, envelope: PokerActionEnvelope) -> Result<bool, Vec<u8>> {
        if envelope.table_id != self.table_id || envelope.hand_id != self.hand_id {
            return Ok(false);
        }

        let seen_key = (envelope.from, envelope.sequence);
        if !self.seen.insert(seen_key) {
            return Ok(false);
        }

        let current_state = self.current_state_tuple()?;
        if current_state != envelope.expected_state {
            return Err(format!(
                "state mismatch before action: local={current_state:?}, remote_expected={:?}",
                envelope.expected_state
            )
            .into_bytes());
        }

        self.apply_action(envelope.action)?;
        Ok(true)
    }

    pub fn current_hand(&self) -> Option<&PokerHand> {
        self.table.get_current_hand()
    }

    fn current_hand_mut(&mut self) -> Result<&mut PokerHand, Vec<u8>> {
        self.table
            .get_current_hand_mut()
            .ok_or_else(|| b"hand not started".to_vec())
    }

    fn current_state_tuple(&self) -> Result<(usize, usize, u8), Vec<u8>> {
        self.table
            .get_current_hand()
            .map(|hand| hand.get_current_state().to_tuple())
            .ok_or_else(|| b"hand not started".to_vec())
    }

    fn apply_action(&mut self, action: PokerAction) -> Result<(), Vec<u8>> {
        let hand = self.current_hand_mut()?;

        match action {
            PokerAction::SubmitShuffledDeck { player, deck } => {
                hand.submit_shuffled_deck(player, deck)
            }
            PokerAction::SubmitSmallBlind { player } => hand.submit_small_blind(player),
            PokerAction::SubmitBigBlind { player } => hand.submit_big_blind(player),
            PokerAction::SubmitPlayerCards { player, cards } => {
                hand.submit_player_cards(player, cards).map(|_| ())
            }
            PokerAction::SubmitCommunityCards {
                player,
                round,
                cards,
            } => hand
                .submit_community_cards(player, round, cards)
                .map(|_| ()),
            PokerAction::SubmitShowdownCards { player, cards } => {
                hand.submit_player_cards_showdown(player, cards).map(|_| ())
            }
            PokerAction::SubmitBet { player, amount } => hand.submit_bet(player, amount),
            PokerAction::SubmitPublicKey {
                player,
                public_key,
                traces,
            } => hand.submit_public_key(player, public_key, traces),
        }
    }
}
