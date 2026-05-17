//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use alloy_primitives::Keccak256;

use crate::util::{compressed_public_key_from_signing_key, make_signing_key_from_seed};

pub const PRIVATE_POKER_DERIVATION_MESSAGE: &str = "Welcome to Private Poker!\n\n\
Sign this message to generate your secure in-game communication key. \n\n\
This key is used to encrypt your peer-to-peer WebRTC signals and verify your actions at the table.\n\n\
This request will not trigger a blockchain transaction or cost any gas.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlsIdentity {
    pub private_key_hex: String,
    pub public_key_hex: String,
}

pub fn derive_private_poker_bls_identity(
    ethereum_private_key: &str,
) -> Result<BlsIdentity, String> {
    derive_private_poker_bls_identity_with_message(
        ethereum_private_key,
        PRIVATE_POKER_DERIVATION_MESSAGE,
    )
}

pub fn derive_private_poker_bls_identity_with_message(
    ethereum_private_key: &str,
    message: &str,
) -> Result<BlsIdentity, String> {
    let signature_material = wallet_signature_material(ethereum_private_key, message)?;
    let seed = keccak256(&signature_material);
    let signing_key = make_signing_key_from_seed(&seed).map_err(str::to_string)?;
    let public_key = compressed_public_key_from_signing_key(&signing_key);

    Ok(BlsIdentity {
        private_key_hex: hex::encode(seed),
        public_key_hex: hex::encode(public_key),
    })
}

pub fn wallet_signature_material(
    ethereum_private_key: &str,
    message: &str,
) -> Result<Vec<u8>, String> {
    let private_key = decode_hex_32(ethereum_private_key)?;
    let mut bytes = Vec::with_capacity(private_key.len() + message.len());
    bytes.extend_from_slice(&private_key);
    bytes.extend_from_slice(message.as_bytes());
    Ok(keccak256(&bytes).to_vec())
}

fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], String> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(value, &mut bytes)
        .map_err(|err| format!("expected 32-byte Ethereum private key hex: {err}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivation_message_matches_frontend_text() {
        assert!(PRIVATE_POKER_DERIVATION_MESSAGE.starts_with("Welcome to Private Poker!"));
        assert!(
            PRIVATE_POKER_DERIVATION_MESSAGE
                .contains("This request will not trigger a blockchain transaction")
        );
    }

    #[test]
    fn decodes_private_key_with_or_without_prefix() {
        let key = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        assert_eq!(decode_hex_32(key).expect("key")[0], 0);
        assert_eq!(decode_hex_32(&format!("0x{key}")).expect("key")[31], 31);
    }

    #[test]
    fn derives_public_key_from_wallet_material() {
        let identity = (1u8..=u8::MAX)
            .find_map(|candidate| {
                let mut key = [0u8; 32];
                key[31] = candidate;
                derive_private_poker_bls_identity(&format!("0x{}", hex::encode(key))).ok()
            })
            .expect("at least one deterministic test key should map to a valid signing key");

        assert_eq!(identity.private_key_hex.len(), 64);
        assert_eq!(identity.public_key_hex.len(), 192);
    }
}
