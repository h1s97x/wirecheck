// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Core types, rule-set schema and static validation for wirecheck.
//!
//! This crate is platform-free: embedding it does not pull in any probe
//! implementation, network client or OS-specific code.

pub mod model;
pub mod rules;
pub mod validate;

#[cfg(test)]
mod tests;

pub use model::{
    Evidence, Finding, Outcome, ProbeMethod, Redaction, Report, REPORT_SCHEMA_VERSION,
};
pub use rules::{
    parse_duration_str, ActionKind, Defaults, ProbeSpec, RedactPolicy, RemediationSpec,
    ReportPolicy, Risk, RollbackSpec, RuleError, RuleSet, ACTION_IDS, PROBE_KINDS,
    SCHEMA_VERSION,
};
pub use validate::{validate, Diagnostic, Severity, ValidationReport};
