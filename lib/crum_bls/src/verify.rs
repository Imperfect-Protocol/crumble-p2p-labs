//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::collections::HashSet;

/// Verification of signatures and unmasking
use bls12_381::{Bls12, G1Affine, G2Affine, G2Prepared};
use pairing::{
    MultiMillerLoop,
    group::{Curve, Group},
};

use crate::{
    hash_to_curve::hash_to_curve,
    types::{PublicKey, Signature},
};

/// Verifies that message has been signed by signing key corresponding to public key.
pub fn verify(message: &[u8], pk: &PublicKey, sig: &Signature) -> bool {
    let h = hash_to_curve(message).to_affine();

    // e(sig, G1) * e(h, -PK) == 1
    // Using BLS12-381 standard pairing check
    let is_valid = Bls12::multi_miller_loop(&[
        (&G1Affine::from(*sig), &G2Affine::generator().into()),
        (&G1Affine::from(h), &(-G2Affine::from(*pk)).into()),
    ])
    .final_exponentiation()
    .is_identity();

    is_valid.into()
}

/// Verifies that "masked" data has been "unmasked" with signing key
/// corresponding to public key.
pub fn verify_unmasking(masked: G1Affine, unmasked: G1Affine, pk: G2Affine) -> bool {
    let g2_gen = G2Affine::generator();
    Bls12::multi_miller_loop(&[
        (&unmasked, &G2Affine::from(pk).into()),
        (&masked, &(-G2Affine::from(g2_gen)).into()),
    ])
    .final_exponentiation()
    .is_identity()
    .into()
}

/// Verifies that "masked_before" data has been shuffled into "masked_after"
/// data with signing key corresponding to public key.
///
/// This is slow brute-force O(N^2) algorithm.
///
pub fn verify_shuffle(
    masked_before: &[G1Affine],
    masked_after: &[G1Affine],
    pk: &G2Affine,
) -> Result<(), &'static str> {
    if masked_before.len() < masked_after.len() {
        return Err("Masked before must at least same length as masked after");
    }

    let pk_prepared = G2Prepared::from(*pk);
    let neg_g2_gen = -G2Affine::generator();
    let neg_g2_prepared = G2Prepared::from(neg_g2_gen);

    let mut available_before = masked_before.to_vec();

    for point_after in masked_after {
        let mut matched_index = None;

        for (i, point_before) in available_before.iter().enumerate() {
            // e(card_after, -G2) * e(card_before, PK) == 1
            let is_match: bool = Bls12::multi_miller_loop(&[
                (point_after, &neg_g2_prepared),
                (point_before, &pk_prepared),
            ])
            .final_exponentiation()
            .is_identity()
            .into();

            if is_match {
                matched_index = Some(i);
                break;
            }
        }

        match matched_index {
            Some(idx) => {
                available_before.remove(idx);
            }
            None => {
                return Err("Cryptographic forgery detected");
            }
        }
    }

    Ok(())
}

pub struct ShuffleTrace {
    pub after_index: usize,
    pub claimed_before_index: usize,
}

/// Verifies that "masked_before" data has been shuffled into "masked_after"
/// data with signing key corresponding to public key.
///
/// This is efficient O(M) algorithm using only single Final Exponentiation call.
///
pub fn verify_shuffle_traced(
    masked_before: &[G1Affine],
    masked_after: &[G1Affine],
    pk: &G2Affine,
    traces: &[ShuffleTrace], // Only M traces submitted
) -> Result<(), &'static str> {
    let pk_prepared = G2Prepared::from(*pk);
    let neg_g2_gen = -G2Affine::generator();
    let neg_g2_prepared = G2Prepared::from(neg_g2_gen);

    // 1. THE BIJECTION CHECK
    let mut used_before_indices = HashSet::new();

    // Create a vector to hold all pairing terms for the batched Miller Loop.
    // Each trace adds 2 terms: one for the card after, one for the card before.
    let mut miller_loop_terms = Vec::with_capacity(traces.len() * 2);

    for trace in traces {
        // Prevent out-of-bounds panics
        if trace.after_index >= masked_after.len()
            || trace.claimed_before_index >= masked_before.len()
        {
            return Err("Trace index out of bounds");
        }

        // Ensure no two outputs point to the same input card
        if !used_before_indices.insert(trace.claimed_before_index) {
            return Err("Duplicate input index! Cheater attempted to clone a card.");
        }

        let point_after = &masked_after[trace.after_index];
        let point_before = &masked_before[trace.claimed_before_index];

        // Push the tuples for this specific trace into the batch array
        miller_loop_terms.push((point_after, &neg_g2_prepared));
        miller_loop_terms.push((point_before, &pk_prepared));
    }

    // 2. THE O(M) BATCHED MILLER LOOP
    // We run the Miller loop over all pairs at once, then do a SINGLE final exponentiation.
    let is_valid: bool = Bls12::multi_miller_loop(&miller_loop_terms)
        .final_exponentiation()
        .is_identity()
        .into();

    if !is_valid {
        return Err("Cryptographic forgery: The batched trace verification failed.");
    }

    Ok(())
}
