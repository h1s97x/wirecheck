// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Observation and assertion contract for M1.
//!
//! A probe reports only what it observed; the `expect` table holds the
//! judgement. Keeping them separate lets the same probe be reused with
//! different thresholds.

use crate::model::{Evidence, Outcome, ProbeMethod};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observation {
    /// Machine-readable values consumed by `expect` and by report redaction.
    pub fields: BTreeMap<String, String>,
    pub elapsed_ms: u64,
    pub method: ProbeMethod,
    /// Set when the probe could not produce an observation at all.
    pub error: Option<String>,
}

impl Observation {
    pub fn observed() -> Self {
        Self::default()
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            error: Some(message.into()),
            ..Self::default()
        }
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    pub fn using(mut self, method: ProbeMethod) -> Self {
        self.method = method;
        self
    }

    pub fn into_evidence(self) -> Evidence {
        Evidence {
            fields: self.fields,
        }
    }
}

fn scalar(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => text.clone(),
        toml::Value::Integer(number) => number.to_string(),
        toml::Value::Float(number) => number.to_string(),
        toml::Value::Boolean(flag) => flag.to_string(),
        other => other.to_string(),
    }
}

/// Evaluate the `expect` table against one observation.
///
/// Unknown assertion keys produce a warning rather than being ignored, so a
/// typo can never make a broken check look healthy.
pub fn evaluate_expect(
    expect: &BTreeMap<String, toml::Value>,
    observation: &Observation,
) -> (Outcome, String) {
    if let Some(error) = &observation.error {
        return (Outcome::Error, format!("probe failed: {error}"));
    }
    if expect.is_empty() {
        return (Outcome::Pass, "no assertions declared".to_string());
    }

    let mut failures = Vec::new();
    let mut warnings = Vec::new();

    for (key, expected) in expect {
        let want = scalar(expected);
        match key.as_str() {
            "status" => {
                let got = observation
                    .fields
                    .get("status")
                    .cloned()
                    .unwrap_or_default();
                if got != want {
                    failures.push(format!("status is `{got}`, expected `{want}`"));
                }
            }
            "min_answers" => {
                let threshold = expected.as_integer().unwrap_or_default();
                let got = observation
                    .fields
                    .get("answers")
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or_default();
                if got < threshold {
                    failures.push(format!("answers {got} < {threshold}"));
                }
            }
            "packet_loss_below" => {
                let threshold = expected
                    .as_float()
                    .unwrap_or_else(|| expected.as_integer().unwrap_or_default() as f64);
                let got = observation
                    .fields
                    .get("packet_loss")
                    .and_then(|value| value.parse::<f64>().ok())
                    .unwrap_or(1.0);
                if got >= threshold {
                    failures.push(format!("packet loss {got} >= {threshold}"));
                }
            }
            "body_contains" => {
                let got = observation.fields.get("body").cloned().unwrap_or_default();
                if !got.contains(&want) {
                    failures.push(format!("response body does not contain `{want}`"));
                }
            }
            "body_not_contains" => {
                let got = observation.fields.get("body").cloned().unwrap_or_default();
                if got.contains(&want) {
                    failures.push(format!("response body contains `{want}`"));
                }
            }
            other => match observation.fields.get(other) {
                Some(got) if *got != want => {
                    failures.push(format!("{other} is `{got}`, expected `{want}`"));
                }
                Some(_) => {}
                None => warnings.push(format!("unrecognised assertion `{other}`")),
            },
        }
    }

    if !failures.is_empty() {
        (Outcome::Fail, failures.join("; "))
    } else if !warnings.is_empty() {
        (
            Outcome::Warn,
            format!("{} (unrecognised assertions)", warnings.join("; ")),
        )
    } else {
        (Outcome::Pass, "all assertions passed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_table(source: &str) -> BTreeMap<String, toml::Value> {
        let value: toml::Value = toml::from_str(source).unwrap();
        value
            .as_table()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    #[test]
    fn a_failed_probe_is_an_error() {
        let (outcome, message) = evaluate_expect(
            &expect_table("status = \"ok\""),
            &Observation::failed("socket timeout"),
        );
        assert_eq!(outcome, Outcome::Error);
        assert!(message.contains("socket timeout"));
    }

    #[test]
    fn status_and_numeric_assertions_are_checked() {
        let rules = expect_table("status = 200\nmin_answers = 1");
        let observation = Observation::observed()
            .with("status", "200")
            .with("answers", "2");
        assert_eq!(evaluate_expect(&rules, &observation).0, Outcome::Pass);

        let observation = Observation::observed()
            .with("status", "503")
            .with("answers", "0");
        assert_eq!(evaluate_expect(&rules, &observation).0, Outcome::Fail);
    }

    #[test]
    fn body_and_loss_assertions_are_checked() {
        let rules = expect_table("body_not_contains = \"unavailable\"\npacket_loss_below = 0.5");
        let observation = Observation::observed()
            .with("body", "all good")
            .with("packet_loss", "0.0");
        assert_eq!(evaluate_expect(&rules, &observation).0, Outcome::Pass);

        let observation = Observation::observed()
            .with("body", "service unavailable")
            .with("packet_loss", "1.0");
        assert_eq!(evaluate_expect(&rules, &observation).0, Outcome::Fail);
    }

    #[test]
    fn unknown_assertions_warn_instead_of_passing_silently() {
        let (outcome, message) = evaluate_expect(
            &expect_table("statuz = \"ok\""),
            &Observation::observed().with("status", "ok"),
        );
        assert_eq!(outcome, Outcome::Warn);
        assert!(message.contains("statuz"));
    }

    #[test]
    fn observation_becomes_redactable_evidence() {
        let evidence = Observation::observed()
            .with("client_ip", "192.0.2.10")
            .into_evidence();
        assert_eq!(evidence.fields["client_ip"], "192.0.2.10");
    }
}
