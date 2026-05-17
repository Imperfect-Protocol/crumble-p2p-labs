//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use crate::{
    poker_state::{POKER_HOLDEM_ROUNDS, PokerHandStateEnum},
    poker_table::PokerTable,
};

use super::poker_deck::PokerDeck;
use bls12_381::Scalar;
use crum_bls::{
    hash_to_curve::hash_to_curve, lagrange, sign, util::make_public_key_from_signing_key, verify,
};
use ff::Field;
use itertools::Itertools;
use pairing::group::Curve;

#[test]
fn test_lifecycle() {
    // --- 1. SETUP ---
    let mut rng = rand::thread_rng();

    // Player A (Dealer) and Player B (Player)
    let sk_a = Scalar::random(&mut rng);
    let sk_b = Scalar::random(&mut rng);

    // Public Keys in G2 for the Miller Loop "Audit"
    let pk_a_g2 = make_public_key_from_signing_key(&sk_a);
    let pk_b_g2 = make_public_key_from_signing_key(&sk_b);

    // The "Ace of Spades" base point
    let card_base = hash_to_curve(b"AS").to_affine();

    // --- 2. SHUFFLE (Commutative Masking) ---
    // Player A masks the card
    let masked_a = sign::mask(card_base, sk_a);

    // Player B masks Player A's result
    let masked_b = sign::mask(masked_a, sk_b);
    // At this point, masked_b = Card * sk_a * sk_b

    // --- 3. BETTING (Threshold Consensus) ---
    let bet_message = b"Player B bets 10 USDC";

    // Both players sign the bet to reach consensus
    let sig_a = sign::sign(bet_message, sk_a);
    let sig_b = sign::sign(bet_message, sk_b);

    // Combine into a threshold signature (2-of-2)
    // For this test, we treat them as IDs 1 and 2
    let shares = vec![(1, sig_a), (2, sig_b)];
    let combined_bet_sig = lagrange::combine(&shares).expect("Failed to combine signatures");

    let pub_shares = vec![(1, pk_a_g2), (2, pk_b_g2)];
    let master_pk = lagrange::recover(&pub_shares).expect("Failed to recover master public key");

    // Verify that bet message was signed by all participants
    assert!(
        verify::verify(bet_message, &master_pk, &combined_bet_sig),
        "Failed to verify bet message"
    );

    // --- 4. DEALING (The Stateless Audit) ---
    // To reveal the card to Player B, Player A must first "peel" their layer
    let unmasked_by_a = sign::unmask(masked_b, sk_a);

    // The Referee (Stylus Contract) verifies Player A was honest
    // It checks: e(Unmasked, G2) == e(Masked, PK_A)
    let audit_passed = verify::verify_unmasking(masked_b, unmasked_by_a, pk_a_g2);
    assert!(audit_passed, "Player A's unmasking audit failed!");

    // Finally, Player B peels their own layer to see the card
    let final_card = sign::unmask(unmasked_by_a, sk_b);

    // Verification: The final point should be the original Ace of Spades
    assert_eq!(final_card, card_base, "The final card point is corrupted!");

    println!("Sovereign Deal Complete: Bet signed and card audited successfully.");
}

#[test]
fn test_poker() {
    let mut rng = rand::thread_rng();

    let sk_1 = Scalar::random(&mut rng);
    let sk_2 = Scalar::random(&mut rng);

    // Player 1 table create (Heads-up/Hold-em)
    // Player 2 join

    // Player 1 turn (shuffle)

    // Player 1 starts with fresh deck of cards...
    let poker_deck = PokerDeck::new();
    let mut masked_deck = poker_deck.masked_cards();

    // ...masks them and shuffles
    masked_deck.mask(sk_1);
    masked_deck.shuffle(&mut rng);

    // Player 1 commits onchain masked/shuffled deck

    // Player 2 turn (shuffle)

    // Player 2 repeats masking and shuffling...
    masked_deck.mask(sk_2);
    masked_deck.shuffle(&mut rng);

    // Player 2 commits onchain masked/shuffled deck

    // Now we have shuffled deck of cards that no one knows which card is which
    // because when player 1 shuffled masked cards, player 2 wouldn't know which
    // card ended up at which position, and then player 2 also masked and shuffled
    // and player 1 can no longer tell which card is which, because all cards are
    // doubly masked by signing key of player 1 and player 2. Cool thing about BLS
    // is that signing is commutative, so we can unmask later on in any order, which
    // is critical for Poker game.
    let deck_hash = masked_deck.hash();

    // Both players sign the exact same flat hash
    let sig_1 = sign::sign(&deck_hash, sk_1);
    let sig_2 = sign::sign(&deck_hash, sk_2);

    // Combine into the Master Signature
    let bls_signature = lagrange::combine(&[(1, sig_1), (2, sig_2)]).expect("Should combine");

    // Players commit signed deck hash onchain (in case game was fully off-chain)

    let pk_1 = make_public_key_from_signing_key(&sk_1);
    let pk_2 = make_public_key_from_signing_key(&sk_2);

    let master_pk =
        lagrange::recover(&[(1, pk_1), (2, pk_2)]).expect("Failed to recover master key");

    // We can verify that deck matches the hash signed and committed onchain by players

    assert!(
        verify::verify(&deck_hash, &master_pk, &bls_signature),
        "Failed to verify deck"
    );

    // ^^^ (Optional) should whole Poker game happen off-chain

    // Player 1 turn (small blind)
    // Player 2 turn (big blind)

    // Posting blinds
    // The players agree off-chain that P1 posts 5 (SB) and P2 posts 10 (BB).
    let blind_state = b"STATE_1: POT=15, P1=-5, P2=-10";

    let blind_sig_1 = sign::sign(blind_state, sk_1);
    let blind_sig_2 = sign::sign(blind_state, sk_2);

    // They combine signatures to prove they both committed to this pot size
    let blinds_master_sig = lagrange::combine(&[(1, blind_sig_1), (2, blind_sig_2)])
        .expect("Failed to combine blind signatures");

    assert!(
        verify::verify(blind_state, &master_pk, &blinds_master_sig),
        "Failed to verify blinds consensus"
    );

    let mut p1_dealt_cards = masked_deck.deal(2);
    let mut p2_dealt_cards = masked_deck.deal(2);

    // normally there would be flop (3 cards), turn (1 card), river (1 card)
    // but just for demo we deal all 5
    let mut community_dealt_cards = masked_deck.deal(5);

    // Each player "peels" away their layer of masking from all cards delt
    // except cards delt to them-selves
    // In case of heads-up: Player 2 unpeels Player 1 cards
    // and then Player 1 unpeels Player 2 cards.
    p1_dealt_cards.unmask(sk_2);
    p2_dealt_cards.unmask(sk_1);

    // At this stage cards require last unpeel, and that must be done by
    // target player, so that no other player will see fully revealed cards.
    // Player 1 unpeels their own cards, and Player 2 unpeels their own cards.
    p1_dealt_cards.unmask(sk_1);
    p2_dealt_cards.unmask(sk_2);

    // An community cards are unmasked by everyone.
    community_dealt_cards.unmask(sk_1);
    community_dealt_cards.unmask(sk_2);

    // Decipher from G1 points to actual poker cards

    let p1_hole_cards = poker_deck.unmasked_cards(&p1_dealt_cards);
    let p2_hole_cards = poker_deck.unmasked_cards(&p2_dealt_cards);
    let community_cards = poker_deck.unmasked_cards(&community_dealt_cards);

    assert!(
        matches!(p1_hole_cards[0], Some(_)),
        "Player 1 Card 0 did not unmask correctly!"
    );
    assert!(
        matches!(p1_hole_cards[1], Some(_)),
        "Player 1 Card 1 did not unmask correctly!"
    );

    assert!(
        matches!(p2_hole_cards[0], Some(_)),
        "Player 2 Card 0 did not unmask correctly!"
    );
    assert!(
        matches!(p2_hole_cards[1], Some(_)),
        "Player 2 Card 1 did not unmask correctly!"
    );

    let p1_hole_cards_str = p1_hole_cards
        .into_iter()
        .map(|c| c.unwrap().to_string())
        .join(", ");

    let p2_hole_cards_str = p2_hole_cards
        .into_iter()
        .map(|c| c.unwrap().to_string())
        .join(", ");

    let community_cards_str = community_cards
        .into_iter()
        .map(|c| c.unwrap().to_string())
        .join(", ");

    println!("Player 1's Hole Cards are: {}", p1_hole_cards_str);
    println!("Player 2's Hole Cards are: {}", p2_hole_cards_str);
    println!("Community Cards are: {}", community_cards_str);
}

#[test]
fn test_poker_table() {
    let mut rng = rand::thread_rng();

    let sk_1 = Scalar::random(&mut rng);
    let sk_2 = Scalar::random(&mut rng);

    let mut shuffle_trace_1 = None;
    let mut shuffle_trace_2 = None;

    let mut poker_table = PokerTable::new(2, POKER_HOLDEM_ROUNDS);

    poker_table.join(1);
    poker_table.join(2);

    poker_table.start_hand(100, 10).unwrap();

    // Player 1 shuffles
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Shuffle {
                player: 0,
                is_dealer: true
            }
        ));

        let mut deck = hand.get_poker_deck().masked_cards();
        deck.mask(sk_1);
        shuffle_trace_1.replace(deck.shuffle_traced(&mut rng));

        println!("Player 1 shuffles deck");

        hand.submit_shuffled_deck(0, deck).unwrap();
    }

    // Player 2 shuffles
    // Note: after this neither plyer will know which card is which.
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        // At this point player 1 will know which shuffled card is which,
        // because they know their private signing key. However player 2 will not
        // know which card is which.
        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Shuffle {
                player: 1,
                is_dealer: false
            }
        ));

        let mut deck = hand.get_shuffled_deck().clone();
        deck.mask(sk_2);
        shuffle_trace_2.replace(deck.shuffle_traced(&mut rng));

        println!("Player 2 shuffles deck");

        hand.submit_shuffled_deck(1, deck).unwrap();
    }

    // Player 1 posts small blind
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        // After all players shuffled, state should progress to posting blinds
        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::SmallBlind { player: 0 }
        ));

        println!("Player 1 posts small blind");

        hand.submit_small_blind(0).unwrap();
    }

    // Player 2 posts big blind
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::BigBlind { player: 1 }
        ));

        println!("Player 2 posts big blind");

        hand.submit_big_blind(1).unwrap();
    }

    // Player 1 unmasks hole cards of player 2
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskHoleCards { player: 0 }
        ));

        let mut cards = hand.get_player_cards().clone();
        cards[1].unmask(sk_1);

        println!("Player 1 unmasks hole cards of Player 2");

        hand.submit_player_cards(0, cards).unwrap();
    }

    // Player 2 unmasks hole cards of player 1
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskHoleCards { player: 1 }
        ));

        let mut cards = hand.get_player_cards().clone();
        cards[0].unmask(sk_2);

        println!("Player 2 unmasks hole cards of Player 1");

        hand.submit_player_cards(1, cards).unwrap();
    }

    // Player 1 unmasks own cards and bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 0,
                player: 0
            }
        ));

        let mut cards = hand.get_player_cards().clone();
        cards[0].unmask(sk_1);

        let p1_cards = hand.get_poker_deck().unmasked_cards(&cards[0]);

        let p1_cards_str = p1_cards
            .into_iter()
            .map(|c| c.unwrap().to_string())
            .join(", ");

        // Player 1 cannot see player 2's cards as they are still masked by player 2 key
        let p2_cards = hand.get_poker_deck().unmasked_cards(&cards[1]);
        assert!(p2_cards.iter().all(|c| c.is_none()));

        println!("Player 1's Hole Cards are: {}", p1_cards_str);

        hand.submit_bet(0, 10).unwrap();
    }

    // Player 2 unmasks own cards and bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 0,
                player: 1
            }
        ));

        let mut cards = hand.get_player_cards().clone();
        cards[1].unmask(sk_2);

        let p2_cards = hand.get_poker_deck().unmasked_cards(&cards[1]);

        let p2_cards_str = p2_cards
            .into_iter()
            .map(|c| c.unwrap().to_string())
            .join(", ");

        // Player 2 cannot see player 1's cards as they are still masked by player 1 key
        let p1_cards = hand.get_poker_deck().unmasked_cards(&cards[0]);
        assert!(p1_cards.iter().all(|c| c.is_none()));

        println!("Player 2's Hole Cards are: {}", p2_cards_str);

        hand.submit_bet(1, 10).unwrap();
    }

    // Player 1 unmasks community cards
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskCommunityCards {
                round: 1,
                player: 0
            }
        ));

        let mut cards = hand.get_community_cards(1).cloned().unwrap();
        cards.unmask(sk_1);

        // community cards are also masked by player 2
        let community_cards = hand.get_poker_deck().unmasked_cards(&cards);
        assert!(community_cards.iter().all(|c| c.is_none()));

        println!("Player 1 unmasks community cards");

        hand.submit_community_cards(0, 1, cards).unwrap();
    }

    // Player 2 unmasks community cards
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskCommunityCards {
                round: 1,
                player: 1
            }
        ));

        let mut cards = hand.get_community_cards(1).cloned().unwrap();
        cards.unmask(sk_2);

        println!("Player 2 unmasks community cards");

        hand.submit_community_cards(1, 1, cards).unwrap();
    }

    // Flop
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        let cards = hand.get_community_cards(1).cloned().unwrap();

        let community_cards = hand.get_poker_deck().unmasked_cards(&cards);

        let community_cards_str = community_cards
            .into_iter()
            .map(|c| c.unwrap().to_string())
            .join(", ");

        println!("Community Cards (Flop) are: {}", community_cards_str);
    }

    // Player 1 bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 1,
                player: 0
            }
        ));

        println!("Player 1 bets");

        hand.submit_bet(0, 10).unwrap();
    }

    // Player 2 bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 1,
                player: 1
            }
        ));

        println!("Player 2 bets");

        hand.submit_bet(1, 10).unwrap();
    }

    // Player 1 unmasks community cards
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskCommunityCards {
                round: 2,
                player: 0
            }
        ));

        let mut cards = hand.get_community_cards(2).cloned().unwrap();
        cards.unmask(sk_1);

        // community cards are also masked by player 2
        let community_cards = hand.get_poker_deck().unmasked_cards(&cards);
        assert!(community_cards.iter().all(|c| c.is_none()));

        println!("Player 1 unmasks community cards");

        hand.submit_community_cards(0, 2, cards).unwrap();
    }

    // Player 2 unmasks community cards
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskCommunityCards {
                round: 2,
                player: 1
            }
        ));

        let mut cards = hand.get_community_cards(2).cloned().unwrap();
        cards.unmask(sk_2);

        println!("Player 2 unmasks community cards");

        hand.submit_community_cards(1, 2, cards).unwrap();
    }

    // Turn
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        let cards = hand.get_community_cards(2).cloned().unwrap();

        let community_cards = hand.get_poker_deck().unmasked_cards(&cards);

        let community_cards_str = community_cards
            .into_iter()
            .map(|c| c.unwrap().to_string())
            .join(", ");

        println!("Community Cards (Turn) are: {}", community_cards_str);
    }

    // Player 1 bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 2,
                player: 0
            }
        ));

        println!("Player 1 bets");

        hand.submit_bet(0, 10).unwrap();
    }

    // Player 2 bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 2,
                player: 1
            }
        ));

        println!("Player 2 bets");

        hand.submit_bet(1, 10).unwrap();
    }

    // Player 1 unmasks community cards
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskCommunityCards {
                round: 3,
                player: 0
            }
        ));

        let mut cards = hand.get_community_cards(3).cloned().unwrap();
        cards.unmask(sk_1);

        // community cards are also masked by player 2
        let community_cards = hand.get_poker_deck().unmasked_cards(&cards);
        assert!(community_cards.iter().all(|c| c.is_none()));

        println!("Player 1 unmasks community cards");

        hand.submit_community_cards(0, 3, cards).unwrap();
    }

    // Player 2 unmasks community cards
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskCommunityCards {
                round: 3,
                player: 1
            }
        ));

        let mut cards = hand.get_community_cards(3).cloned().unwrap();
        cards.unmask(sk_2);

        println!("Player 2 unmasks community cards");

        hand.submit_community_cards(1, 3, cards).unwrap();
    }

    // River
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        let cards = hand.get_community_cards(3).cloned().unwrap();

        let community_cards = hand.get_poker_deck().unmasked_cards(&cards);

        let community_cards_str = community_cards
            .into_iter()
            .map(|c| c.unwrap().to_string())
            .join(", ");

        println!("Community Cards (River) are: {}", community_cards_str);
    }

    // Player 1 bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 3,
                player: 0
            }
        ));

        println!("Player 1 bets");

        hand.submit_bet(0, 10).unwrap();
    }

    // Player 2 bets
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Bet {
                round: 3,
                player: 1
            }
        ));

        println!("Player 2 bets");

        hand.submit_bet(1, 10).unwrap();
    }

    // Player 1 unmasks hole cards for showdown
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskShowdown { player: 0 }
        ));

        let mut cards = hand.get_player_cards().clone();
        cards[0].unmask(sk_1);

        println!("Player 1 unmasks their own cards for showdown");

        hand.submit_player_cards_showdown(0, cards).unwrap();
    }

    // Player 2 unmasks hole cards for showdown
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::UnmaskShowdown { player: 1 }
        ));

        let mut cards = hand.get_player_cards().clone();
        cards[1].unmask(sk_2);

        println!("Player 2 unmasks their own cards for showdown");

        hand.submit_player_cards_showdown(1, cards).unwrap();
    }

    // Player 1 submits public key
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::SubmitPublicKey { player: 0 }
        ));

        let pk = make_public_key_from_signing_key(&sk_1);

        println!("Player 1 submits their ephemeral public key");

        hand.submit_public_key(0, pk, shuffle_trace_1.unwrap())
            .unwrap();
    }

    // Player 2 submits public key
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::SubmitPublicKey { player: 1 }
        ));

        let pk = make_public_key_from_signing_key(&sk_2);

        println!("Player 2 submits their ephemeral public key");

        hand.submit_public_key(1, pk, shuffle_trace_2.unwrap())
            .unwrap();
    }

    // Hand finished
    {
        let hand = poker_table.get_current_hand_mut().unwrap();

        assert!(matches!(
            hand.get_current_state().to_enum(),
            PokerHandStateEnum::Finished
        ));

        println!("Finished");
    }
}
