//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use alloy_primitives::keccak256;
use bls12_381::G2Projective;
use pairing::group::Curve;
use rand::{RngCore, rngs::OsRng};

use crate::util::{
    PUBLIC_KEY_COMPRESSED_LEN, make_public_key_from_compressed_slice, make_signing_key_from_seed,
};

pub fn encrypt_signal_for_peer(
    recipient_public_key_hex: &str,
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    let recipient_public_key = decode_public_key_hex(recipient_public_key_hex)?;
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let ephemeral_key = make_signing_key_from_seed(&seed).map_err(str::to_string)?;
    let ephemeral_public_key = (G2Projective::generator() * ephemeral_key)
        .to_affine()
        .to_compressed();
    let shared_point = (G2Projective::from(recipient_public_key) * ephemeral_key)
        .to_affine()
        .to_compressed();
    let ciphertext = xor_with_keccak_stream(&shared_point, plaintext);

    let mut encrypted = Vec::with_capacity(PUBLIC_KEY_COMPRESSED_LEN + ciphertext.len());
    encrypted.extend_from_slice(&ephemeral_public_key);
    encrypted.extend_from_slice(&ciphertext);
    Ok(encrypted)
}

pub fn decrypt_signal_from_peer(
    private_key_seed_hex: &str,
    encrypted: &[u8],
) -> Result<Vec<u8>, String> {
    if encrypted.len() < PUBLIC_KEY_COMPRESSED_LEN {
        return Err("encrypted signal is shorter than ephemeral public key".to_string());
    }
    let private_seed = decode_private_seed_hex(private_key_seed_hex)?;
    let private_key = make_signing_key_from_seed(&private_seed).map_err(str::to_string)?;
    let ephemeral_public_key =
        make_public_key_from_compressed_slice(&encrypted[..PUBLIC_KEY_COMPRESSED_LEN])
            .map_err(str::to_string)?;
    let shared_point = (G2Projective::from(ephemeral_public_key) * private_key)
        .to_affine()
        .to_compressed();
    Ok(xor_with_keccak_stream(
        &shared_point,
        &encrypted[PUBLIC_KEY_COMPRESSED_LEN..],
    ))
}

fn decode_public_key_hex(value: &str) -> Result<crate::types::PublicKey, String> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    let bytes = hex::decode(value).map_err(|err| format!("invalid public key hex: {err}"))?;
    make_public_key_from_compressed_slice(&bytes).map_err(str::to_string)
}

fn decode_private_seed_hex(value: &str) -> Result<[u8; 32], String> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(value, &mut bytes)
        .map_err(|err| format!("expected 32-byte private key seed hex: {err}"))?;
    Ok(bytes)
}

fn xor_with_keccak_stream(shared_point: &[u8], input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut counter = 0u64;
    for chunk in input.chunks(32) {
        let mut material = Vec::with_capacity(shared_point.len() + 8);
        material.extend_from_slice(shared_point);
        material.extend_from_slice(&counter.to_be_bytes());
        let mask = keccak256(material);
        output.extend(chunk.iter().zip(mask).map(|(byte, mask)| byte ^ mask));
        counter += 1;
    }
    output
}
