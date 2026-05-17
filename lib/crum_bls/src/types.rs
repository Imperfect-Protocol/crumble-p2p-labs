//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use bls12_381::{G1Affine, G2Affine, Scalar};

pub type SigningKey = Scalar;
pub type Signature = G1Affine;
pub type PublicKey = G2Affine;
