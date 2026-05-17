//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{
    collections::BTreeMap,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

pub const DEFAULT_TABLE_REGISTRY_PATH: &str = "crum_p2p_tables.tsv";

pub fn default_registry_path() -> PathBuf {
    PathBuf::from(DEFAULT_TABLE_REGISTRY_PATH)
}

pub fn remember_table(path: &Path, table_name: &str, table_id: u64) -> Result<(), String> {
    let mut entries = read_entries(path)?;
    entries.insert(table_name.to_string(), table_id);
    write_entries(path, &entries)
}

pub fn resolve_table(path: &Path, table_name: &str) -> Result<Option<u64>, String> {
    Ok(read_entries(path)?.get(table_name).copied())
}

fn read_entries(path: &Path) -> Result<BTreeMap<String, u64>, String> {
    let input = match fs::read_to_string(path) {
        Ok(input) => input,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(err) => return Err(format!("failed to read `{}`: {err}", path.display())),
    };

    let mut entries = BTreeMap::new();
    for (index, line) in input.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((id, name)) = line.split_once('\t') else {
            return Err(format!(
                "invalid table registry line {} in `{}`",
                index + 1,
                path.display()
            ));
        };
        let table_id = id.parse::<u64>().map_err(|err| {
            format!(
                "invalid table id `{id}` on line {} in `{}`: {err}",
                index + 1,
                path.display()
            )
        })?;
        entries.insert(name.to_string(), table_id);
    }

    Ok(entries)
}

fn write_entries(path: &Path, entries: &BTreeMap<String, u64>) -> Result<(), String> {
    let mut output = String::new();
    output.push_str("# table_id\tname\n");
    for (name, table_id) in entries {
        if name.contains('\n') || name.contains('\t') {
            return Err(format!(
                "table name cannot contain tabs or newlines: `{name}`"
            ));
        }
        output.push_str(&format!("{table_id}\t{name}\n"));
    }

    let mut file = fs::File::create(path)
        .map_err(|err| format!("failed to write `{}`: {err}", path.display()))?;
    file.write_all(output.as_bytes())
        .map_err(|err| format!("failed to write `{}`: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_and_resolves_table_names() {
        let path = std::env::temp_dir().join(format!(
            "crum_p2p_tables_{}_{}.tsv",
            std::process::id(),
            "remember"
        ));
        let _ = fs::remove_file(&path);

        remember_table(&path, "Player 1 Game", 1234567).expect("remember");
        assert_eq!(
            resolve_table(&path, "Player 1 Game").expect("resolve"),
            Some(1234567)
        );

        remember_table(&path, "Player 1 Game", 7654321).expect("update");
        assert_eq!(
            resolve_table(&path, "Player 1 Game").expect("resolve updated"),
            Some(7654321)
        );

        let _ = fs::remove_file(path);
    }
}
