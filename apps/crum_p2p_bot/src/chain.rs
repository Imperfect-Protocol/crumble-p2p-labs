//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use alloy_network::EthereumWallet;
use alloy_primitives::{Address, Bytes, U256, keccak256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::{TransactionRequest, request::TransactionInput};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{SolCall, sol};
use k256::ecdsa::SigningKey;

sol! {
    interface IPrivatePokerLobby {
        function createTable(uint256 lobby_id, uint256 table_id, string name, uint256 buy_in, bytes annonce_public_key) external;
        function joinTable(uint256 lobby_id, uint256 table_id, bytes annonce_public_key) external;
    }
}

pub const DEFAULT_RPC_URL: &str = "http://localhost:8547";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainConnection {
    pub rpc_url: String,
    pub chain_id: u64,
}

pub async fn connect(rpc_url: Option<&str>) -> Result<ChainConnection, String> {
    let rpc_url = rpc_url.unwrap_or(DEFAULT_RPC_URL).to_string();
    let parsed_url = rpc_url
        .parse()
        .map_err(|err| format!("invalid rpc url `{rpc_url}`: {err}"))?;
    let provider = ProviderBuilder::new().on_http(parsed_url);
    let chain_id = provider
        .get_chain_id()
        .await
        .map_err(|err| format!("failed to connect to `{rpc_url}`: {err}"))?;

    Ok(ChainConnection { rpc_url, chain_id })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCreatedOnChain {
    pub lobby_id: u64,
    pub table_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableJoinedOnChain {
    pub lobby_id: u64,
    pub table_id: u64,
}

pub async fn create_table(
    rpc_url: Option<&str>,
    private_key: &str,
    lobby_address: &str,
    lobby_id: u64,
    table_name: &str,
    buy_in: u64,
    annonce_public_key_hex: &str,
) -> Result<TableCreatedOnChain, String> {
    let table_id = generate_table_id()?;
    let call = IPrivatePokerLobby::createTableCall {
        lobby_id: U256::from(lobby_id),
        table_id: U256::from(table_id),
        name: table_name.to_string(),
        buy_in: U256::from(buy_in),
        annonce_public_key: decode_hex_bytes(annonce_public_key_hex)?.into(),
    };

    send_lobby_transaction(rpc_url, private_key, lobby_address, call.abi_encode()).await?;
    Ok(TableCreatedOnChain { lobby_id, table_id })
}

pub async fn join_table(
    rpc_url: Option<&str>,
    private_key: &str,
    lobby_address: &str,
    lobby_id: u64,
    table_id: u64,
    annonce_public_key_hex: &str,
) -> Result<TableJoinedOnChain, String> {
    let call = IPrivatePokerLobby::joinTableCall {
        lobby_id: U256::from(lobby_id),
        table_id: U256::from(table_id),
        annonce_public_key: decode_hex_bytes(annonce_public_key_hex)?.into(),
    };

    send_lobby_transaction(rpc_url, private_key, lobby_address, call.abi_encode()).await?;
    Ok(TableJoinedOnChain { lobby_id, table_id })
}

async fn send_lobby_transaction(
    rpc_url: Option<&str>,
    private_key: &str,
    lobby_address: &str,
    calldata: Vec<u8>,
) -> Result<(), String> {
    let rpc_url = rpc_url.unwrap_or(DEFAULT_RPC_URL).to_string();
    let parsed_url = rpc_url
        .parse()
        .map_err(|err| format!("invalid rpc url `{rpc_url}`: {err}"))?;
    let from = address_from_private_key(private_key)?;
    let signer = PrivateKeySigner::from_str(private_key)
        .map_err(|err| format!("invalid signing private key: {err}"))?;
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new().wallet(wallet).on_http(parsed_url);

    let to = Address::from_str(lobby_address)
        .map_err(|err| format!("invalid lobby address `{lobby_address}`: {err}"))?;

    let tx = TransactionRequest::default()
        .from(from)
        .to(to)
        .input(TransactionInput::new(Bytes::from(calldata)));

    let receipt = provider
        .send_transaction(tx)
        .await
        .map_err(|err| {
            format!("failed to send signed transaction from {from} via `{rpc_url}`: {err}")
        })?
        .get_receipt()
        .await
        .map_err(|err| format!("failed waiting for transaction receipt: {err}"))?;

    if !receipt.status() {
        return Err(format!(
            "transaction reverted: {:?}",
            receipt.transaction_hash
        ));
    }

    Ok(())
}

pub fn address_from_private_key(private_key: &str) -> Result<Address, String> {
    let key_bytes = decode_fixed_32(private_key)?;
    let signing_key = SigningKey::from_slice(&key_bytes)
        .map_err(|err| format!("invalid secp256k1 private key: {err}"))?;
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let public_key = encoded.as_bytes();
    if public_key.len() != 65 {
        return Err("unexpected secp256k1 public key length".to_string());
    }

    let hash = keccak256(&public_key[1..]);
    Ok(Address::from_slice(&hash[12..]))
}

fn decode_hex_bytes(value: &str) -> Result<Vec<u8>, String> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    hex::decode(value).map_err(|err| format!("invalid hex bytes: {err}"))
}

fn decode_fixed_32(value: &str) -> Result<[u8; 32], String> {
    let value = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(value, &mut bytes)
        .map_err(|err| format!("expected 32-byte private key hex: {err}"))?;
    Ok(bytes)
}

fn generate_table_id() -> Result<u64, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system time before Unix epoch: {err}"))?;
    let micros = now.as_micros();
    let id = micros % 9_000_000_000_000_000 + 1_000_000;
    u64::try_from(id).map_err(|err| format!("table id overflow: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_anvil_account_address() {
        let address = address_from_private_key(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .expect("address");

        assert_eq!(
            address,
            Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()
        );
    }
}
