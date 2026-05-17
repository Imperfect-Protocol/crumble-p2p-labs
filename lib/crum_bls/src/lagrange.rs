//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use bls12_381::{G1Projective, G2Projective, Scalar};
use pairing::group::Curve;

use crate::types::{PublicKey, Signature};

pub fn combine(shares: &[(u64, Signature)]) -> Result<Signature, &'static str> {
    let mut combined = G1Projective::identity();
    let x = shares
        .iter()
        .map(|(label, _)| Scalar::from(*label))
        .collect::<Vec<Scalar>>();
    for i in 0..shares.len() {
        let s = shares[i].1;
        let x_i = x[i];
        let mut l = Scalar::one();
        for j in 0..x.len() {
            if i != j {
                let x_j = x[j];
                let d = (x_j - x_i)
                    .invert()
                    .into_option()
                    .ok_or("Failed to invert denominator")?;
                l *= x_j * d;
            }
        }
        combined += G1Projective::from(s) * l;
    }
    Ok(combined.to_affine())
}

pub fn recover(shares: &[(u64, PublicKey)]) -> Result<PublicKey, &'static str> {
    let mut a = G2Projective::identity();
    for i in 0..shares.len() {
        let (label_i, pk_i) = shares[i];
        let x_i = Scalar::from(label_i);
        let mut l = Scalar::one();
        for j in 0..shares.len() {
            if i != j {
                let label_j = shares[j].0;
                let x_j = Scalar::from(label_j);
                let d = (x_j - x_i)
                    .invert()
                    .into_option()
                    .ok_or("Failed to invert denominator")?;
                l *= x_j * d;
            }
        }
        a += G2Projective::from(pk_i) * l;
    }
    Ok(a.to_affine())
}
