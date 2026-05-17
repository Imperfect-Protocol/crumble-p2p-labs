//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use bls12_381::G1Affine;
use pairing::group::Curve;

use crate::{
    hash_to_curve::hash_to_curve,
    types::{Signature, SigningKey},
};

pub fn sign(data: &[u8], k: SigningKey) -> Signature {
    let mut p = hash_to_curve(data);
    p *= k;
    p.to_affine()
}

pub fn mask(g1: G1Affine, k: SigningKey) -> G1Affine {
    let p = g1 * k;
    p.to_affine()
}

pub fn unmask(g1: G1Affine, k: SigningKey) -> G1Affine {
    let i = k.invert().expect("Failed to invert");
    let u = g1 * i;
    u.to_affine()
}
