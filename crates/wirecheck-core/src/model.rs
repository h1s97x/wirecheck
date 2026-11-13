// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Runtime data contract. M0 defines the types; the runner arrives in M1.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version of the serialized report format, independent of the rule-set schema.
pub const REPORT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Fail,
    Warn,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Redaction {
    None,
    Mask,
    #[default]
    Hash,
}

/// Records which step of the ICMP degradation chain produced an observation.
/// Degradation must never be silent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeMethod {
    #[default]
    Native,
    SystemFallback,
    TcpApproximation,
}

/// Structured, machine-readable evidence. Kept separate from the human-facing
/// message so it can be redacted and consumed programmatically.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

impl Evidence {
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.fields.insert(key.into(), value.into());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub probe_id: String,
    pub outcome: Outcome,
    pub message: String,
    pub evidence: Evidence,
    /// Reference to a registered remediation action ID, never inline code.
    #[serde(default)]
    pub remediation: Option<String>,
    #[serde(default)]
    pub method: ProbeMethod,
    #[serde(default)]
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u16,
    pub rule_set: String,
    /// M0 uses a caller-supplied string; a real clock is wired in M1.
    pub generated_at: String,
    #[serde(default)]
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn new(rule_set: impl Into<String>, generated_at: impl Into<String>) -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            rule_set: rule_set.into(),
            generated_at: generated_at.into(),
            findings: Vec::new(),
        }
    }

    /// Process exit code for a completed run.
    pub fn exit_code(&self) -> i32 {
        if self
            .findings
            .iter()
            .any(|f| matches!(f.outcome, Outcome::Fail | Outcome::Error))
        {
            1
        } else {
            0
        }
    }
}
