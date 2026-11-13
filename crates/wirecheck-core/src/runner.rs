// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Concurrent, dependency-ordered runner for M1.
//!
//! The runner is platform-free: probes are supplied through `ProbeExecutor`,
//! so embedding this crate does not pull in any network or OS-specific code.

use crate::model::{Evidence, Finding, Outcome, ProbeMethod, Report};
use crate::observation::{evaluate_expect, Observation};
use crate::rules::{ProbeSpec, RuleSet};
use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RunContext {
    pub timeout: Duration,
    pub retries: u32,
    pub concurrency: usize,
}

impl RunContext {
    pub fn from_rules(rules: &RuleSet) -> Self {
        Self {
            timeout: rules.defaults.timeout,
            retries: rules.defaults.retries,
            concurrency: rules.defaults.concurrency.max(1),
        }
    }
}

/// Implemented by the built-in probe registry. `Send + Sync` is required so
/// one executor can be shared by all worker threads.
pub trait ProbeExecutor: Send + Sync {
    fn observe(&self, spec: &ProbeSpec, context: &RunContext) -> Observation;
}

/// Execute every probe, one dependency level at a time, with bounded
/// concurrency. A probe that fails to observe marks its dependents as
/// `skipped`; the run continues.
pub fn run<E: ProbeExecutor>(rules: &RuleSet, executor: &E) -> Report {
    let context = RunContext::from_rules(rules);
    let index: HashMap<&str, usize> = rules
        .probes
        .iter()
        .enumerate()
        .map(|(position, probe)| (probe.id.as_str(), position))
        .collect();
    let mut outcomes: Vec<Option<Outcome>> = vec![None; rules.probes.len()];
    let mut report = Report::new(rules.name.clone(), now_rfc3339());

    for layer in topo_layers(rules) {
        let mut collected: Vec<(usize, Finding)> = Vec::new();

        for chunk in layer.chunks(context.concurrency) {
            let jobs: Vec<(usize, ProbeSpec, bool)> = chunk
                .iter()
                .map(|&position| {
                    let spec = substitute(&rules.probes[position], &rules.vars);
                    let blocked = rules.probes[position].depends_on.iter().any(|dependency| {
                        index
                            .get(dependency.as_str())
                            .and_then(|&parent| outcomes[parent])
                            .is_some_and(|outcome| {
                                matches!(outcome, Outcome::Error | Outcome::Skipped)
                            })
                    });
                    (position, spec, blocked)
                })
                .collect();

            std::thread::scope(|scope| {
                let (sender, receiver) = mpsc::channel();
                for (position, spec, blocked) in &jobs {
                    if *blocked {
                        let _ = sender.send((*position, skipped_finding(spec, rules)));
                        continue;
                    }
                    let sender = sender.clone();
                    let context = &context;
                    scope.spawn(move || {
                        let mut observation = Observation::failed("probe did not run");
                        for _ in 0..=context.retries {
                            observation = executor.observe(spec, context);
                            if observation.error.is_none() {
                                break;
                            }
                        }
                        let (outcome, message) = evaluate_expect(&spec.expect, &observation);
                        let finding = Finding {
                            probe_id: spec.id.clone(),
                            outcome,
                            message,
                            evidence: Evidence {
                                fields: observation.fields,
                            },
                            remediation: remediation_for(rules, &spec.id),
                            method: observation.method,
                            elapsed_ms: observation.elapsed_ms,
                        };
                        let _ = sender.send((*position, finding));
                    });
                }
                drop(sender);
                for (position, finding) in receiver {
                    collected.push((position, finding));
                }
            });
        }

        collected.sort_by_key(|(position, _)| *position);
        for (position, finding) in collected {
            outcomes[position] = Some(finding.outcome);
            report.findings.push(finding);
        }
    }

    report
}

fn skipped_finding(spec: &ProbeSpec, rules: &RuleSet) -> Finding {
    Finding {
        probe_id: spec.id.clone(),
        outcome: Outcome::Skipped,
        message: "skipped: a dependency failed".to_string(),
        evidence: Evidence::default(),
        remediation: remediation_for(rules, &spec.id),
        method: ProbeMethod::default(),
        elapsed_ms: 0,
    }
}

fn remediation_for(rules: &RuleSet, probe_id: &str) -> Option<String> {
    rules
        .remediations
        .iter()
        .find(|remediation| remediation.target == probe_id)
        .map(|remediation| remediation.id.clone())
}

/// Group probe indices into dependency levels. `validate` already rejects
/// cycles; the bounded relaxation here is only a defensive fallback.
pub fn topo_layers(rules: &RuleSet) -> Vec<Vec<usize>> {
    let total = rules.probes.len();
    let index: HashMap<&str, usize> = rules
        .probes
        .iter()
        .enumerate()
        .map(|(position, probe)| (probe.id.as_str(), position))
        .collect();
    let mut level = vec![0usize; total];

    for _ in 0..total {
        let mut changed = false;
        for (position, probe) in rules.probes.iter().enumerate() {
            for dependency in &probe.depends_on {
                if let Some(&parent) = index.get(dependency.as_str()) {
                    if level[parent] + 1 > level[position] {
                        level[position] = level[parent] + 1;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    let highest = level.iter().copied().max().unwrap_or(0);
    (0..=highest)
        .map(|current| {
            (0..total)
                .filter(|&position| level[position] == current)
                .collect::<Vec<usize>>()
        })
        .filter(|layer| !layer.is_empty())
        .collect()
}

/// Replace `{{ vars.NAME }}` in a probe's parameters and assertions before it
/// is handed to an executor. There is no expression evaluation.
pub fn substitute(spec: &ProbeSpec, vars: &BTreeMap<String, String>) -> ProbeSpec {
    let mut result = spec.clone();
    result.with = spec
        .with
        .iter()
        .map(|(key, value)| (key.clone(), substitute_value(value, vars)))
        .collect();
    result.expect = spec
        .expect
        .iter()
        .map(|(key, value)| (key.clone(), substitute_value(value, vars)))
        .collect();
    result
}

fn substitute_value(value: &toml::Value, vars: &BTreeMap<String, String>) -> toml::Value {
    match value {
        toml::Value::String(text) => toml::Value::String(substitute_text(text, vars)),
        toml::Value::Array(items) => toml::Value::Array(
            items
                .iter()
                .map(|item| substitute_value(item, vars))
                .collect(),
        ),
        toml::Value::Table(table) => {
            let mut result = toml::map::Map::new();
            for (key, item) in table {
                result.insert(key.clone(), substitute_value(item, vars));
            }
            toml::Value::Table(result)
        }
        other => other.clone(),
    }
}

pub fn substitute_text(text: &str, vars: &BTreeMap<String, String>) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let tail = &rest[start..];
        if let Some(end) = tail.find("}}") {
            let token = tail[2..end].trim();
            match token.strip_prefix("vars.") {
                Some(name) => match vars.get(name.trim()) {
                    Some(value) => output.push_str(value),
                    None => output.push_str(&tail[..end + 2]),
                },
                None => output.push_str(&tail[..end + 2]),
            }
            rest = &tail[end + 2..];
        } else {
            output.push_str(tail);
            rest = "";
        }
    }
    output.push_str(rest);
    output
}

/// Real UTC clock for `Report.generated_at` (M0 used a caller-supplied string).
pub fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    unix_to_rfc3339(seconds)
}

fn unix_to_rfc3339(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let era_base = days + 719_468;
    let era = if era_base >= 0 {
        era_base
    } else {
        era_base - 146_096
    } / 146_097;
    let day_of_era = era_base - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        remainder / 3600,
        (remainder % 3600) / 60,
        remainder % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fake;

    impl ProbeExecutor for Fake {
        fn observe(&self, spec: &ProbeSpec, _context: &RunContext) -> Observation {
            if spec.id == "broken" {
                return Observation::failed("boom");
            }
            let target = spec
                .with
                .get("target")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            Observation::observed()
                .with("status", "ok")
                .with("target", target)
        }
    }

    struct Flaky {
        attempts: AtomicUsize,
    }

    impl ProbeExecutor for Flaky {
        fn observe(&self, _spec: &ProbeSpec, _context: &RunContext) -> Observation {
            if self.attempts.fetch_add(1, Ordering::SeqCst) < 2 {
                Observation::failed("transient")
            } else {
                Observation::observed().with("status", "ok")
            }
        }
    }

    fn parse(source: &str) -> RuleSet {
        RuleSet::from_toml(source).unwrap()
    }

    #[test]
    fn layers_respect_dependencies() {
        let rules = parse(
            r#"
version = 1
name = "layers"
[[probes]]
id = "a"
type = "tcp"
[[probes]]
id = "b"
type = "tcp"
depends_on = ["a"]
[[probes]]
id = "c"
type = "tcp"
depends_on = ["b"]
"#,
        );
        assert_eq!(topo_layers(&rules), vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn a_failed_probe_skips_its_dependents() {
        let rules = parse(
            r#"
version = 1
name = "skip"
[[probes]]
id = "broken"
type = "tcp"
expect = { status = "ok" }
[[probes]]
id = "after"
type = "tcp"
depends_on = ["broken"]
expect = { status = "ok" }
"#,
        );
        let report = run(&rules, &Fake);
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].probe_id, "broken");
        assert_eq!(report.findings[0].outcome, Outcome::Error);
        assert_eq!(report.findings[1].outcome, Outcome::Skipped);
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn variables_are_substituted_before_observation() {
        let rules = parse(
            r#"
version = 1
name = "vars"
[vars]
gateway = "192.0.2.1"
[[probes]]
id = "p"
type = "tcp"
expect = { status = "ok" }
[probes.with]
target = "{{ vars.gateway }}"
"#,
        );
        let report = run(&rules, &Fake);
        assert_eq!(report.findings[0].outcome, Outcome::Pass);
        assert_eq!(report.findings[0].evidence.fields["target"], "192.0.2.1");
    }

    #[test]
    fn failed_attempts_are_retried() {
        let rules = parse(
            r#"
version = 1
name = "retry"
[defaults]
retries = 2
[[probes]]
id = "flaky"
type = "tcp"
expect = { status = "ok" }
"#,
        );
        let executor = Flaky {
            attempts: AtomicUsize::new(0),
        };
        let report = run(&rules, &executor);
        assert_eq!(report.findings[0].outcome, Outcome::Pass);
        assert_eq!(executor.attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn timestamp_is_a_real_utc_clock() {
        assert_eq!(unix_to_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(unix_to_rfc3339(1_609_459_200), "2021-01-01T00:00:00Z");
        assert!(now_rfc3339().ends_with('Z'));
    }
}
