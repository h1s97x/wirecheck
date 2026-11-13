// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Static validation. Never touches the network and reports every problem in
//! one pass, so CI output is actionable without repeated edit-run cycles.

use crate::rules::{
    ProbeSpec, Risk, RuleSet, ACTION_IDS, PROBE_KINDS, REPORT_FORMATS, SCHEMA_VERSION,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub rule_set: String,
    pub diagnostics: Vec<Diagnostic>,
    pub probe_count: usize,
    pub remediation_count: usize,
}

impl ValidationReport {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    pub fn is_ok(&self) -> bool {
        self.error_count() == 0
    }

    pub fn exit_code(&self) -> i32 {
        if self.is_ok() {
            0
        } else {
            2
        }
    }
}

fn err(
    out: &mut Vec<Diagnostic>,
    code: &str,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    out.push(Diagnostic {
        severity: Severity::Error,
        code: code.to_string(),
        path: path.into(),
        message: message.into(),
    });
}

fn warn(
    out: &mut Vec<Diagnostic>,
    code: &str,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    out.push(Diagnostic {
        severity: Severity::Warning,
        code: code.to_string(),
        path: path.into(),
        message: message.into(),
    });
}

/// Validate a parsed rule set without performing any I/O.
pub fn validate(rules: &RuleSet) -> ValidationReport {
    let mut out: Vec<Diagnostic> = Vec::new();

    if rules.version != SCHEMA_VERSION {
        err(
            &mut out,
            "E001",
            "version",
            format!(
                "unsupported schema version {}, expected {SCHEMA_VERSION}",
                rules.version
            ),
        );
    }
    if rules.name.trim().is_empty() {
        err(&mut out, "E002", "name", "rule set name must not be empty");
    }
    if rules.defaults.concurrency == 0 {
        err(
            &mut out,
            "E003",
            "defaults.concurrency",
            "concurrency must be at least 1",
        );
    }
    if rules.defaults.retries > 10 {
        warn(
            &mut out,
            "W001",
            "defaults.retries",
            "retries above 10 will make a run very slow",
        );
    }
    for format in &rules.reports.formats {
        if !REPORT_FORMATS.contains(&format.as_str()) {
            err(
                &mut out,
                "E010",
                "reports.formats",
                format!("unknown report format `{format}`"),
            );
        }
    }
    if rules.reports.output_dir.trim().is_empty() {
        err(
            &mut out,
            "E018",
            "reports.output_dir",
            "output directory must not be empty",
        );
    }
    if rules.probes.is_empty() {
        err(
            &mut out,
            "E004",
            "probes",
            "a rule set must declare at least one probe",
        );
    }
    for name in rules.vars.keys() {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            err(
                &mut out,
                "E017",
                format!("vars.{name}"),
                "variable names must be non-empty alphanumeric/underscore strings",
            );
        }
    }

    let mut seen_ids: BTreeSet<String> = BTreeSet::new();
    let mut duplicate_ids = false;
    for (index, probe) in rules.probes.iter().enumerate() {
        if probe.id.trim().is_empty() {
            err(
                &mut out,
                "E005",
                format!("probes[{index}].id"),
                "probe id must not be empty",
            );
        }
        if !seen_ids.insert(probe.id.clone()) {
            duplicate_ids = true;
            err(
                &mut out,
                "E006",
                format!("probes[{index}].id"),
                format!("duplicate probe id `{}`", probe.id),
            );
        }
        if !PROBE_KINDS.contains(&probe.kind.as_str()) {
            err(
                &mut out,
                "E007",
                format!("probes[{index}].type"),
                format!("unknown probe kind `{}`", probe.kind),
            );
        }
        for dependency in &probe.depends_on {
            if *dependency == probe.id {
                err(
                    &mut out,
                    "E009",
                    format!("probes[{index}].depends_on"),
                    format!("probe `{}` depends on itself", probe.id),
                );
            }
        }
    }

    let known_ids: BTreeSet<String> = rules.probes.iter().map(|p| p.id.clone()).collect();
    for (index, probe) in rules.probes.iter().enumerate() {
        for dependency in &probe.depends_on {
            if !known_ids.contains(dependency) {
                err(
                    &mut out,
                    "E008",
                    format!("probes[{index}].depends_on"),
                    format!("unknown dependency `{dependency}`"),
                );
            }
        }
        for (key, value) in probe.with.iter().chain(probe.expect.iter()) {
            let mut refs = Vec::new();
            collect_var_refs(value, &mut refs);
            for name in refs {
                if !rules.vars.contains_key(&name) {
                    err(
                        &mut out,
                        "E012",
                        format!("probes[{index}].{key}"),
                        format!("undefined variable `vars.{name}`"),
                    );
                }
            }
        }
    }

    // Cycle detection is only meaningful when ids are unique.
    if !duplicate_ids && has_cycle(&rules.probes) {
        err(
            &mut out,
            "E011",
            "probes.depends_on",
            "dependency graph contains a cycle",
        );
    }

    let mut seen_remediation_ids: BTreeSet<String> = BTreeSet::new();
    for (index, remedy) in rules.remediations.iter().enumerate() {
        if !known_ids.contains(&remedy.target) {
            err(
                &mut out,
                "E013",
                format!("remediations[{index}].for"),
                format!("remediation targets unknown probe `{}`", remedy.target),
            );
        }
        if !ACTION_IDS.contains(&remedy.action.as_str()) {
            err(
                &mut out,
                "E014",
                format!("remediations[{index}].action"),
                format!("unknown action id `{}`", remedy.action),
            );
        }
        if let Some(rollback) = &remedy.rollback {
            if !ACTION_IDS.contains(&rollback.action.as_str()) {
                err(
                    &mut out,
                    "E015",
                    format!("remediations[{index}].rollback.action"),
                    format!("unknown rollback action id `{}`", rollback.action),
                );
            }
        }
        if remedy.id.trim().is_empty() {
            err(
                &mut out,
                "E019",
                format!("remediations[{index}].id"),
                "remediation id must not be empty",
            );
        }
        if !seen_remediation_ids.insert(remedy.id.clone()) {
            err(
                &mut out,
                "E016",
                format!("remediations[{index}].id"),
                format!("duplicate remediation id `{}`", remedy.id),
            );
        }
        if !remedy.needs_confirm && remedy.risk != Risk::Low {
            warn(
                &mut out,
                "W003",
                format!("remediations[{index}].needs_confirm"),
                "a non-low-risk action should require confirmation",
            );
        }
    }

    ValidationReport {
        rule_set: rules.name.clone(),
        probe_count: rules.probes.len(),
        remediation_count: rules.remediations.len(),
        diagnostics: out,
    }
}

/// Collect `{{ vars.NAME }}` references from any nested TOML value.
fn collect_var_refs(value: &toml::Value, out: &mut Vec<String>) {
    match value {
        toml::Value::String(text) => {
            let mut rest = text.as_str();
            while let Some(start) = rest.find("{{") {
                let after = &rest[start + 2..];
                let Some(end) = after.find("}}") else { break };
                let expression = after[..end].trim();
                if let Some(name) = expression.strip_prefix("vars.") {
                    out.push(name.trim().to_string());
                }
                rest = &after[end + 2..];
            }
        }
        toml::Value::Array(items) => {
            for item in items {
                collect_var_refs(item, out);
            }
        }
        toml::Value::Table(table) => {
            for item in table.values() {
                collect_var_refs(item, out);
            }
        }
        _ => {}
    }
}

/// Kahn's algorithm over the `depends_on` graph.
fn has_cycle(probes: &[ProbeSpec]) -> bool {
    let mut indegree: BTreeMap<String, usize> = probes
        .iter()
        .map(|probe| (probe.id.clone(), 0))
        .collect();
    let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for probe in probes {
        for dependency in &probe.depends_on {
            if indegree.contains_key(dependency) {
                *indegree
                    .get_mut(&probe.id)
                    .expect("probe id is present in indegree") += 1;
                edges
                    .entry(dependency.clone())
                    .or_default()
                    .push(probe.id.clone());
            }
        }
    }

    let mut queue: Vec<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(node) = queue.pop() {
        visited += 1;
        if let Some(next) = edges.get(&node) {
            for dependent in next {
                let degree = indegree
                    .get_mut(dependent)
                    .expect("edge target is present in indegree");
                *degree -= 1;
                if *degree == 0 {
                    queue.push(dependent.clone());
                }
            }
        }
    }
    visited != indegree.len()
}
