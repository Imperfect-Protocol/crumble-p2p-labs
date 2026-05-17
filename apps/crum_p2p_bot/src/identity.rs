//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

pub use crum_bls::identity::{
    BlsIdentity, PRIVATE_POKER_DERIVATION_MESSAGE as DERIVATION_MESSAGE,
    derive_private_poker_bls_identity as derive_bls_identity,
};
