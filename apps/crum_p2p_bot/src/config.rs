//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{env, fs, path::PathBuf, str::FromStr};

use crum_net::TopologyPolicy;

pub const DEFAULT_CONFIG_PATH: &str = "crum_p2p_bot.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotConfig {
    pub rpc_url: Option<String>,
    pub private_key: Option<String>,
    pub mode: BotMode,
    pub lobby_id: Option<u64>,
    pub table_id: Option<u64>,
    pub table_name: Option<String>,
    pub buy_in: Option<u64>,
    pub max_players: Option<u8>,
    pub topology: TopologyPolicy,
    pub contracts: ContractAddresses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotMode {
    Create,
    Join,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContractAddresses {
    pub lobby: Option<String>,
    pub signal: Option<String>,
    pub verify_shuffle: Option<String>,
    pub verify_unmasking: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOptions {
    pub config_path: PathBuf,
    pub rpc_url: Option<String>,
    pub private_key: Option<String>,
    pub mode: Option<BotMode>,
    pub lobby_id: Option<u64>,
    pub table_id: Option<u64>,
    pub table_name: Option<String>,
    pub buy_in: Option<u64>,
    pub max_players: Option<u8>,
    pub topology: Option<TopologyPolicy>,
    pub contracts: ContractAddresses,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            rpc_url: None,
            private_key: None,
            mode: BotMode::Join,
            lobby_id: None,
            table_id: None,
            table_name: None,
            buy_in: None,
            max_players: None,
            topology: TopologyPolicy::Ring,
            contracts: ContractAddresses::default(),
        }
    }
}

impl FromStr for BotMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match normalized(value).as_str() {
            "create" => Ok(Self::Create),
            "join" => Ok(Self::Join),
            _ => Err(format!("unknown mode `{value}`")),
        }
    }
}

impl BotConfig {
    pub fn load(cli: CliOptions) -> Result<Self, String> {
        let mut config = if cli.config_path.exists() {
            Self::from_toml_str(&fs::read_to_string(&cli.config_path).map_err(|err| {
                format!(
                    "failed to read config `{}`: {err}",
                    cli.config_path.display()
                )
            })?)?
        } else {
            Self::default()
        };

        config.apply_cli(cli);
        config.apply_env_defaults();
        Ok(config)
    }

    pub fn from_toml_str(input: &str) -> Result<Self, String> {
        let mut config = Self::default();
        let mut section = String::new();

        for (index, raw_line) in input.lines().enumerate() {
            let line = strip_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_string();
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("invalid config line {}: `{}`", index + 1, raw_line));
            };

            config.set_config_value(&section, key.trim(), value.trim())?;
        }

        Ok(config)
    }

    fn set_config_value(&mut self, section: &str, key: &str, value: &str) -> Result<(), String> {
        match (section, key) {
            ("", "rpc_url") | ("network", "rpc_url") => self.rpc_url = Some(parse_string(value)?),
            ("", "private_key") | ("wallet", "private_key") => {
                self.private_key = Some(parse_string(value)?)
            }
            ("", "mode") | ("table", "mode") => self.mode = parse_string(value)?.parse()?,
            ("", "lobby_id") | ("table", "lobby_id") => self.lobby_id = Some(parse_u64(value)?),
            ("", "table_id") | ("table", "table_id") => self.table_id = Some(parse_u64(value)?),
            ("", "table_name") | ("table", "table_name") | ("table", "name") => {
                self.table_name = Some(parse_string(value)?)
            }
            ("", "buy_in") | ("table", "buy_in") => self.buy_in = Some(parse_u64(value)?),
            ("", "max_players") | ("table", "max_players") => {
                self.max_players = Some(parse_u8(value)?)
            }
            ("", "topology") | ("network", "topology") | ("table", "topology") => {
                self.topology = parse_topology(&parse_string(value)?)?
            }
            ("contracts", "lobby") | ("contracts", "lobby_address") => {
                self.contracts.lobby = Some(parse_string(value)?)
            }
            ("contracts", "signal") | ("contracts", "signal_address") => {
                self.contracts.signal = Some(parse_string(value)?)
            }
            ("contracts", "verify_shuffle") | ("contracts", "verify_shuffle_address") => {
                self.contracts.verify_shuffle = Some(parse_string(value)?)
            }
            ("contracts", "verify_unmasking") | ("contracts", "verify_unmasking_address") => {
                self.contracts.verify_unmasking = Some(parse_string(value)?)
            }
            _ => return Err(format!("unknown config key `{key}` in section `{section}`")),
        }
        Ok(())
    }

    fn apply_cli(&mut self, cli: CliOptions) {
        if cli.rpc_url.is_some() {
            self.rpc_url = cli.rpc_url;
        }
        if cli.private_key.is_some() {
            self.private_key = cli.private_key;
        }
        if let Some(mode) = cli.mode {
            self.mode = mode;
        }
        if cli.lobby_id.is_some() {
            self.lobby_id = cli.lobby_id;
        }
        if cli.table_id.is_some() {
            self.table_id = cli.table_id;
        }
        if cli.table_name.is_some() {
            self.table_name = cli.table_name;
        }
        if cli.buy_in.is_some() {
            self.buy_in = cli.buy_in;
        }
        if cli.max_players.is_some() {
            self.max_players = cli.max_players;
        }
        if let Some(topology) = cli.topology {
            self.topology = topology;
        }
        self.contracts.merge(cli.contracts);
    }

    fn apply_env_defaults(&mut self) {
        if self.contracts.lobby.is_none() {
            self.contracts.lobby = env::var("PP_LOBBY").ok();
        }
        if self.contracts.signal.is_none() {
            self.contracts.signal = env::var("PP_SIGNAL").ok();
        }
        if self.contracts.verify_shuffle.is_none() {
            self.contracts.verify_shuffle = env::var("PP_VERIFY_SHUFFLE").ok();
        }
        if self.contracts.verify_unmasking.is_none() {
            self.contracts.verify_unmasking = env::var("PP_VERIFY_UNMASKING").ok();
        }
    }
}

impl CliOptions {
    pub fn from_env() -> Result<Self, String> {
        Self::from_args(env::args().skip(1))
    }

    pub fn from_args<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut options = Self {
            config_path: PathBuf::from(DEFAULT_CONFIG_PATH),
            rpc_url: None,
            private_key: None,
            mode: None,
            lobby_id: None,
            table_id: None,
            table_name: None,
            buy_in: None,
            max_players: None,
            topology: None,
            contracts: ContractAddresses::default(),
        };

        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let mut index = 0;
        while index < args.len() {
            let raw_arg = &args[index];
            let (arg, inline_value) = split_inline_arg(raw_arg);
            match arg.as_str() {
                "--create" => options.mode = Some(BotMode::Create),
                "--join" => options.mode = Some(BotMode::Join),
                "--create-table" => {
                    options.mode = Some(BotMode::Create);
                    options.table_name = Some(next_arg_value(&args, &mut index, inline_value)?);
                    if options.buy_in.is_none() {
                        options.buy_in = Some(0);
                    }
                    if options.max_players.is_none() {
                        options.max_players = Some(2);
                    }
                }
                "--join-table" => {
                    options.mode = Some(BotMode::Join);
                    let value = next_arg_value(&args, &mut index, inline_value)?;
                    match parse_u64(&value) {
                        Ok(table_id) => options.table_id = Some(table_id),
                        Err(_) => options.table_name = Some(value),
                    }
                }
                "--config" => {
                    options.config_path =
                        PathBuf::from(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--rpc-url" => {
                    options.rpc_url = Some(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--private-key" => {
                    options.private_key = Some(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--mode" => {
                    options.mode = Some(next_arg_value(&args, &mut index, inline_value)?.parse()?)
                }
                "--lobby" | "--lobby-id" => {
                    options.lobby_id = Some(parse_u64(&next_arg_value(
                        &args,
                        &mut index,
                        inline_value,
                    )?)?)
                }
                "--table-id" => {
                    options.table_id = Some(parse_u64(&next_arg_value(
                        &args,
                        &mut index,
                        inline_value,
                    )?)?)
                }
                "--table-name" => {
                    options.table_name = Some(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--buy-in" => {
                    options.buy_in = Some(parse_u64(&next_arg_value(
                        &args,
                        &mut index,
                        inline_value,
                    )?)?)
                }
                "--max-players" | "--num-players" => {
                    options.max_players =
                        Some(parse_u8(&next_arg_value(&args, &mut index, inline_value)?)?)
                }
                "--topology" => {
                    options.topology = Some(parse_topology(&next_arg_value(
                        &args,
                        &mut index,
                        inline_value,
                    )?)?)
                }
                "--lobby-address" => {
                    options.contracts.lobby = Some(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--signal-address" => {
                    options.contracts.signal =
                        Some(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--verify-shuffle-address" => {
                    options.contracts.verify_shuffle =
                        Some(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--verify-unmasking-address" => {
                    options.contracts.verify_unmasking =
                        Some(next_arg_value(&args, &mut index, inline_value)?)
                }
                "--help" | "-h" => return Err(usage()),
                _ => return Err(format!("unknown argument `{raw_arg}`\n\n{}", usage())),
            }
            index += 1;
        }

        Ok(options)
    }
}

impl ContractAddresses {
    fn merge(&mut self, other: Self) {
        if other.lobby.is_some() {
            self.lobby = other.lobby;
        }
        if other.signal.is_some() {
            self.signal = other.signal;
        }
        if other.verify_shuffle.is_some() {
            self.verify_shuffle = other.verify_shuffle;
        }
        if other.verify_unmasking.is_some() {
            self.verify_unmasking = other.verify_unmasking;
        }
    }
}

pub fn usage() -> String {
    format!(
        "Usage: crum_p2p_bot [--config {DEFAULT_CONFIG_PATH}] [--rpc-url URL] [--private-key HEX] [--create|--join]\n\
         \n\
         Shortcuts: --create-table NAME --lobby N | --join-table TABLE_ID_OR_NAME --lobby N\n\
         \n\
         Table: --lobby-id N --table-id N [--table-name NAME] [--buy-in WEI] [--max-players N|--num-players N]\n\
         Network: [--topology ring|mesh|ring-one-backup|ring-two-backups]\n\
         Contracts: [--lobby-address HEX] [--signal-address HEX] [--verify-shuffle-address HEX] [--verify-unmasking-address HEX]"
    )
}

fn next_value(args: &[String], index: &mut usize) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| "missing value for argument".to_string())
}

fn split_inline_arg(arg: &str) -> (String, Option<String>) {
    arg.split_once('=')
        .map(|(key, value)| (key.to_string(), Some(value.to_string())))
        .unwrap_or_else(|| (arg.to_string(), None))
}

fn next_arg_value(
    args: &[String],
    index: &mut usize,
    inline_value: Option<String>,
) -> Result<String, String> {
    if let Some(value) = inline_value {
        return Ok(value);
    }
    next_value(args, index)
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(before, _)| before)
}

fn parse_string(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        Ok(value[1..value.len() - 1].to_string())
    } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        Ok(value[1..value.len() - 1].to_string())
    } else {
        Ok(value.to_string())
    }
}

fn parse_u64(value: &str) -> Result<u64, String> {
    parse_string(value)?
        .parse()
        .map_err(|err| format!("expected integer `{value}`: {err}"))
}

fn parse_u8(value: &str) -> Result<u8, String> {
    parse_string(value)?
        .parse()
        .map_err(|err| format!("expected u8 `{value}`: {err}"))
}

fn parse_topology(value: &str) -> Result<TopologyPolicy, String> {
    match normalized(value).as_str() {
        "ring" => Ok(TopologyPolicy::Ring),
        "mesh" => Ok(TopologyPolicy::Mesh),
        "ring-one-backup" | "ring-with-one-backup" | "ring1" => Ok(TopologyPolicy::RingOneBackup),
        "ring-two-backups" | "ring-with-two-backups" | "ring2" => {
            Ok(TopologyPolicy::RingTwoBackups)
        }
        _ => Err(format!("unknown topology `{value}`")),
    }
}

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_table_and_contract_config() {
        let config = BotConfig::from_toml_str(
            r#"
            [network]
            rpc_url = "https://sepolia-rollup.arbitrum.io/rpc"
            topology = "ring_two_backups"

            [wallet]
            private_key = "0xabc"

            [table]
            mode = "create"
            lobby_id = 7
            table_id = 42
            table_name = "Friday"
            buy_in = 1000
            max_players = 6

            [contracts]
            lobby = "0x1"
            signal = "0x2"
            verify_shuffle = "0x3"
            verify_unmasking = "0x4"
            "#,
        )
        .expect("config");

        assert_eq!(config.mode, BotMode::Create);
        assert_eq!(config.topology, TopologyPolicy::RingTwoBackups);
        assert_eq!(config.lobby_id, Some(7));
        assert_eq!(config.table_name.as_deref(), Some("Friday"));
        assert_eq!(config.contracts.signal.as_deref(), Some("0x2"));
    }

    #[test]
    fn cli_overrides_config_values() {
        let cli = CliOptions::from_args([
            "--config",
            "missing.toml",
            "--create",
            "--topology",
            "mesh",
            "--max-players",
            "4",
        ])
        .expect("cli");
        let config = BotConfig::load(cli).expect("config");

        assert_eq!(config.mode, BotMode::Create);
        assert_eq!(config.topology, TopologyPolicy::Mesh);
        assert_eq!(config.max_players, Some(4));
    }

    #[test]
    fn join_table_accepts_id_or_name() {
        let by_id = CliOptions::from_args(["--join-table", "12345"]).expect("cli by id");
        assert_eq!(by_id.mode, Some(BotMode::Join));
        assert_eq!(by_id.table_id, Some(12345));
        assert_eq!(by_id.table_name, None);

        let by_name =
            CliOptions::from_args(["--join-table", "Player 1 Game"]).expect("cli by name");
        assert_eq!(by_name.mode, Some(BotMode::Join));
        assert_eq!(by_name.table_id, None);
        assert_eq!(by_name.table_name.as_deref(), Some("Player 1 Game"));
    }
}
