// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Rule-set fingerprint trust store for M3.
//!
//! This is a local SHA-256 allow-list, not an asymmetric signature scheme: it
//! detects changes to a rule set you have already reviewed. Verification of a
//! publisher's identity is out of scope until key management is designed.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const STORE: &str = ".wirecheck/trusted.txt";

pub fn fingerprint(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut output = String::with_capacity(64);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn store_path(root: &Path) -> PathBuf {
    root.join(STORE)
}

fn key(rule_path: &Path) -> String {
    rule_path
        .canonicalize()
        .unwrap_or_else(|_| rule_path.to_path_buf())
        .display()
        .to_string()
}

fn load(root: &Path) -> Result<BTreeMap<String, String>> {
    let path = store_path(root);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("cannot read trust store `{}`", path.display()))?;
    let mut entries = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((digest, path)) = line.split_once('\t') {
            entries.insert(path.to_string(), digest.to_string());
        }
    }
    Ok(entries)
}

pub fn is_trusted(root: &Path, rule_path: &Path, text: &str) -> Result<bool> {
    let entries = load(root)?;
    Ok(entries
        .get(&key(rule_path))
        .is_some_and(|expected| expected == &fingerprint(text)))
}

pub fn trust(root: &Path, rule_path: &Path, text: &str) -> Result<String> {
    let mut entries = load(root)?;
    let digest = fingerprint(text);
    entries.insert(key(rule_path), digest.clone());

    let mut body = String::from("# wirecheck trusted rule sets\n# sha256<TAB>absolute path\n");
    for (path, value) in &entries {
        let _ = writeln!(body, "{value}\t{path}");
    }
    let path = store_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, body)?;
    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_roundtrip_detects_tampering() {
        let root = std::env::temp_dir().join(format!("wirecheck-trust-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let rules = root.join("example.rules.toml");
        fs::create_dir_all(&root).unwrap();
        fs::write(&rules, "version = 1\nname = \"x\"\n").unwrap();

        assert!(!is_trusted(&root, &rules, "version = 1\nname = \"x\"\n").unwrap());
        let digest = trust(&root, &rules, "version = 1\nname = \"x\"\n").unwrap();
        assert_eq!(digest.len(), 64);
        assert!(is_trusted(&root, &rules, "version = 1\nname = \"x\"\n").unwrap());
        assert!(!is_trusted(&root, &rules, "version = 1\nname = \"changed\"\n").unwrap());
        let _ = fs::remove_dir_all(&root);
    }
}
