// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

use crate::model::{Evidence, Finding, Outcome, ProbeMethod, Report};
use crate::rules::{parse_duration_str, RuleSet};
use crate::validate::{validate, Severity};
use std::collections::BTreeSet;
use std::time::Duration;

const HOME: &str = include_str!("../../../rules/home.rules.toml");
const CI_RULES: &str = include_str!("../../../rules/ci.rules.toml");

#[test]
fn example_rule_sets_are_valid() {
    for text in [HOME, CI_RULES] {
        let rules = RuleSet::from_toml(text).expect("example rule set must parse");
        let report = validate(&rules);
        assert!(
            report.is_ok(),
            "unexpected diagnostics: {:?}",
            report.diagnostics
        );
        assert!(report.probe_count > 0);
    }
}

#[test]
fn duration_literals_are_parsed() {
    assert_eq!(parse_duration_str("250ms"), Ok(Duration::from_millis(250)));
    assert_eq!(parse_duration_str("3s"), Ok(Duration::from_secs(3)));
    assert_eq!(parse_duration_str("2m"), Ok(Duration::from_secs(120)));
    assert_eq!(parse_duration_str("1h"), Ok(Duration::from_secs(3600)));
    assert!(parse_duration_str("").is_err());
    assert!(parse_duration_str("abc").is_err());
    assert!(parse_duration_str("10x").is_err());
}

#[test]
fn malformed_toml_is_a_parse_error() {
    assert!(RuleSet::from_toml("version = [oops").is_err());
    assert!(RuleSet::from_toml("name = 'missing version'").is_err());
}

#[test]
fn reports_all_diagnostics_in_one_pass() {
    let text = r#"
version = 1
name = "broken"

[[probes]]
id = "a"
type = "dns"
depends_on = ["missing"]

[[probes]]
id = "b"
type = "icmp"
depends_on = ["a"]

[probes.with]
target = "{{ vars.undefined }}"

[[probes]]
id = "a"
type = "not_a_probe"

[[remediations]]
for = "ghost"
id = "r1"
action = "shell.exec"

[[remediations]]
for = "a"
id = "r2"
action = "dns.set"

[remediations.rollback]
action = "also.unknown"
"#;
    let rules = RuleSet::from_toml(text).expect("syntactically valid TOML");
    let report = validate(&rules);
    let codes: BTreeSet<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();

    for expected in ["E006", "E007", "E008", "E012", "E013", "E014", "E015"] {
        assert!(
            codes.contains(expected),
            "missing diagnostic {expected}: {codes:?}"
        );
    }
    assert!(!report.is_ok());
    assert_eq!(report.exit_code(), 2);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count(),
        report.error_count()
    );
}

#[test]
fn dependency_cycle_is_detected() {
    let text = r#"
version = 1
name = "cyclic"

[[probes]]
id = "a"
type = "dns"
depends_on = ["b"]

[[probes]]
id = "b"
type = "http"
depends_on = ["a"]
"#;
    let rules = RuleSet::from_toml(text).unwrap();
    let report = validate(&rules);
    assert!(
        report.diagnostics.iter().any(|d| d.code == "E011"),
        "expected a cycle diagnostic: {:?}",
        report.diagnostics
    );
}

#[test]
fn unknown_action_id_is_reported_with_its_path() {
    let text = r#"
version = 1
name = "actions"

[[probes]]
id = "p"
type = "proxy"

[[remediations]]
for = "p"
id = "r"
action = "os.reboot"
"#;
    let rules = RuleSet::from_toml(text).unwrap();
    let report = validate(&rules);
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|d| d.code == "E014")
        .expect("unknown action must be reported");
    assert_eq!(diagnostic.path, "remediations[0].action");
    assert!(diagnostic.message.contains("os.reboot"));
}

#[test]
fn report_exit_code_reflects_failures() {
    let mut report = Report::new("demo", "unset");
    assert_eq!(report.schema_version, 1);
    assert_eq!(report.exit_code(), 0);

    report.findings.push(Finding {
        probe_id: "p".to_string(),
        outcome: Outcome::Fail,
        message: "unreachable".to_string(),
        evidence: Evidence::default(),
        remediation: None,
        method: ProbeMethod::Native,
        elapsed_ms: 12,
    });
    assert_eq!(report.exit_code(), 1);
}
