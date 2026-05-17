//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use bls12_381::{G2Projective, Scalar};
use ff::Field;
use pairing::group::Curve;

use crate::types::{PublicKey, Signature, SigningKey};

pub const SIGNING_KEY_LEN: usize = 32;
pub const SIGNATURE_COMPRESSED_LEN: usize = 48;
pub const PUBLIC_KEY_COMPRESSED_LEN: usize = 96;

pub fn make_public_key_from_compressed_slice(data: &[u8]) -> Result<PublicKey, &'static str> {
    if data.len() != PUBLIC_KEY_COMPRESSED_LEN {
        return Err("Len Error");
    }
    let mut bytes = [0u8; PUBLIC_KEY_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    PublicKey::from_compressed(&bytes)
        .into_option()
        .ok_or("Decode Error")
}

pub fn make_signature_from_compressed_slice(data: &[u8]) -> Result<Signature, &'static str> {
    if data.len() != SIGNATURE_COMPRESSED_LEN {
        return Err("Len Error");
    }
    let mut bytes = [0u8; SIGNATURE_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    Signature::from_compressed(&bytes)
        .into_option()
        .ok_or("Decode Error")
}

pub fn make_public_key_from_signing_key(sk: &SigningKey) -> PublicKey {
    (G2Projective::generator() * sk).to_affine()
}

pub fn make_signing_key_from_seed(seed: &[u8]) -> Result<SigningKey, &'static str> {
    if seed.len() != SIGNING_KEY_LEN {
        return Err("Invalid signing key seed length");
    }

    let mut bytes = [0u8; SIGNING_KEY_LEN * 2];
    bytes[..SIGNING_KEY_LEN].copy_from_slice(seed);
    bytes[SIGNING_KEY_LEN..].copy_from_slice(seed);

    let signing_key = Scalar::from_bytes_wide(&bytes);
    if bool::from(signing_key.is_zero()) {
        return Err("Invalid zero signing key seed");
    }

    Ok(signing_key)
}

pub fn compressed_public_key_from_signing_key(sk: &SigningKey) -> [u8; PUBLIC_KEY_COMPRESSED_LEN] {
    make_public_key_from_signing_key(sk).to_compressed()
}
