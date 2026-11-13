// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Declarative rule-set schema.

use crate::model::Redaction;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::time::Duration;

pub const SCHEMA_VERSION: u16 = 1;

/// Built-in probe kinds. M0 declares them; implementations land in M1.
pub const PROBE_KINDS: &[&str] = &[
    "dns",
    "http",
    "icmp",
    "tcp",
    "proxy",
    "interface",
    "traceroute",
    "captive_portal",
];

/// Closed allow-list of remediation action IDs. A rule set can only name one
/// of these; it can never introduce new behaviour.
pub const ACTION_IDS: &[&str] = &["proxy.set", "dns.set", "route.flush", "interface.renew"];

pub const REPORT_FORMATS: &[&str] = &["json", "sarif", "html"];

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("failed to parse rule set: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleSet {
    pub version: u16,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub redact: RedactPolicy,
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    #[serde(default)]
    pub probes: Vec<ProbeSpec>,
    #[serde(default)]
    pub remediations: Vec<RemediationSpec>,
    #[serde(default)]
    pub reports: ReportPolicy,
}

impl RuleSet {
    pub fn from_toml(input: &str) -> Result<Self, RuleError> {
        toml::from_str(input).map_err(|e| RuleError::Parse(e.to_string()))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_timeout", deserialize_with = "parse_duration")]
    pub timeout: Duration,
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            retries: default_retries(),
            concurrency: default_concurrency(),
        }
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(3)
}
fn default_retries() -> u32 {
    1
}
fn default_concurrency() -> usize {
    4
}

/// Parse a duration literal such as `250ms`, `3s`, `2m`, `1h`.
pub fn parse_duration_str(raw: &str) -> Result<Duration, String> {
    let text = raw.trim();
    let split = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, unit) = text.split_at(split);
    if digits.is_empty() {
        return Err(format!("invalid duration `{raw}`"));
    }
    let amount: u64 = digits
        .parse()
        .map_err(|_| format!("invalid duration `{raw}`"))?;
    let multiplier = match unit.trim() {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        other => {
            return Err(format!(
                "unsupported duration unit `{other}` (use ms/s/m/h)"
            ))
        }
    };
    amount
        .checked_mul(multiplier)
        .map(Duration::from_millis)
        .ok_or_else(|| format!("duration overflow `{raw}`"))
}

fn parse_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_duration_str(&raw).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedactPolicy {
    #[serde(default)]
    pub ip: Redaction,
    #[serde(default)]
    pub user: Redaction,
    #[serde(default)]
    pub mac: Redaction,
}

impl Default for RedactPolicy {
    fn default() -> Self {
        Self {
            ip: Redaction::Mask,
            user: Redaction::Hash,
            mac: Redaction::Mask,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProbeSpec {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub with: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub expect: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemediationSpec {
    #[serde(rename = "for")]
    pub target: String,
    pub id: String,
    pub action: String,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
    #[serde(default = "default_risk")]
    pub risk: Risk,
    #[serde(default)]
    pub needs_confirm: bool,
    #[serde(default)]
    pub rollback: Option<RollbackSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Low,
    Medium,
    High,
}

fn default_risk() -> Risk {
    Risk::Medium
}

#[derive(Debug, Clone, Deserialize)]
pub struct RollbackSpec {
    pub action: String,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}

/// Marker for action metadata; the compiled registry lives in builtins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Reversible,
    NonReversible,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportPolicy {
    #[serde(default = "default_formats")]
    pub formats: Vec<String>,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

impl Default for ReportPolicy {
    fn default() -> Self {
        Self {
            formats: default_formats(),
            output_dir: default_output_dir(),
        }
    }
}

fn default_formats() -> Vec<String> {
    vec!["json".to_string(), "html".to_string()]
}

fn default_output_dir() -> String {
    "./reports".to_string()
}
