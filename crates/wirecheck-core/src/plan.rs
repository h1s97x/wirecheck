// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Remediation plans and audit records for M2.
//!
//! A plan is pure data derived from a validated rule set and a completed run.
//! Nothing here touches the host: execution is delegated to an
//! `ActionExecutor` supplied by the caller.

use crate::model::{Outcome, Report};
use crate::rules::{Risk, RuleSet};
use crate::runner::now_rfc3339;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub action: String,
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedAction {
    pub remediation_id: String,
    pub probe_id: String,
    pub action: String,
    pub params: BTreeMap<String, String>,
    pub risk: String,
    pub needs_confirm: bool,
    pub rollback: Option<RollbackPlan>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub rule_set: String,
    pub generated_at: String,
    pub actions: Vec<PlannedAction>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn requires_confirm(&self) -> bool {
        self.actions.iter().any(|action| action.needs_confirm)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub remediation_id: String,
    /// `applied`, `failed`, `blocked` or `rolled_back`.
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub rule_set: String,
    pub generated_at: String,
    pub plan: Plan,
    pub results: Vec<ActionOutcome>,
    pub rollback_results: Vec<ActionOutcome>,
}

/// Implemented outside `wirecheck-core` so this crate stays platform-free.
pub trait ActionExecutor: Send + Sync {
    fn apply(&self, action: &PlannedAction) -> ActionOutcome;
    fn rollback(&self, action: &PlannedAction) -> ActionOutcome;
}

/// Build a dry-run plan: only findings that are not healthy and that reference
/// a registered remediation produce an action.
pub fn build_plan(rules: &RuleSet, report: &Report) -> Plan {
    let mut actions = Vec::new();
    for finding in &report.findings {
        if !matches!(
            finding.outcome,
            Outcome::Fail | Outcome::Error | Outcome::Warn
        ) {
            continue;
        }
        let Some(remediation_id) = &finding.remediation else {
            continue;
        };
        let Some(spec) = rules
            .remediations
            .iter()
            .find(|remediation| &remediation.id == remediation_id)
        else {
            continue;
        };
        actions.push(PlannedAction {
            remediation_id: spec.id.clone(),
            probe_id: spec.target.clone(),
            action: spec.action.clone(),
            params: flatten(&spec.params),
            risk: risk_label(spec.risk).to_string(),
            needs_confirm: spec.needs_confirm,
            rollback: spec.rollback.as_ref().map(|rollback| RollbackPlan {
                action: rollback.action.clone(),
                params: flatten(&rollback.params),
            }),
        });
    }
    Plan {
        rule_set: rules.name.clone(),
        generated_at: report.generated_at.clone(),
        actions,
    }
}

/// Apply a plan and produce a complete audit record. If any action does not
/// succeed, every successfully applied action is rolled back.
pub fn apply_plan<A: ActionExecutor>(plan: &Plan, executor: &A) -> AuditRecord {
    let mut results = Vec::new();
    for action in &plan.actions {
        results.push(executor.apply(action));
    }
    let mut rollback_results = Vec::new();
    if results.iter().any(|result| result.status != "applied") {
        for (index, action) in plan.actions.iter().enumerate() {
            if results[index].status == "applied" {
                rollback_results.push(executor.rollback(action));
            }
        }
    }
    AuditRecord {
        rule_set: plan.rule_set.clone(),
        generated_at: now_rfc3339(),
        plan: plan.clone(),
        results,
        rollback_results,
    }
}

fn risk_label(risk: Risk) -> &'static str {
    match risk {
        Risk::Low => "low",
        Risk::Medium => "medium",
        Risk::High => "high",
    }
}

fn flatten(table: &BTreeMap<String, toml::Value>) -> BTreeMap<String, String> {
    table
        .iter()
        .map(|(key, value)| (key.clone(), scalar(value)))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Evidence, Finding, ProbeMethod};

    struct Executor {
        fail_id: Option<String>,
    }

    impl ActionExecutor for Executor {
        fn apply(&self, action: &PlannedAction) -> ActionOutcome {
            let status = if self.fail_id.as_deref() == Some(action.action.as_str()) {
                "failed"
            } else {
                "applied"
            };
            ActionOutcome {
                remediation_id: action.remediation_id.clone(),
                status: status.to_string(),
                detail: "test".to_string(),
            }
        }

        fn rollback(&self, action: &PlannedAction) -> ActionOutcome {
            ActionOutcome {
                remediation_id: action.remediation_id.clone(),
                status: "rolled_back".to_string(),
                detail: "test".to_string(),
            }
        }
    }

    fn fixture() -> (RuleSet, Report) {
        let rules = RuleSet::from_toml(
            r#"
version = 1
name = "plan-demo"
[[probes]]
id = "a"
type = "tcp"
[[probes]]
id = "b"
type = "tcp"
[[remediations]]
for = "a"
id = "fix_a"
action = "proxy.set"
risk = "low"
needs_confirm = true
[remediations.params]
enabled = false
[remediations.rollback]
action = "proxy.set"
[remediations.rollback.params]
enabled = true
[[remediations]]
for = "b"
id = "fix_b"
action = "dns.set"
risk = "high"
"#,
        )
        .unwrap();
        let mut report = Report::new("plan-demo", "2026-11-14T00:00:00Z");
        for (id, outcome, remediation) in [
            ("a", Outcome::Fail, Some("fix_a")),
            ("b", Outcome::Pass, Some("fix_b")),
        ] {
            report.findings.push(Finding {
                probe_id: id.to_string(),
                outcome,
                message: "demo".to_string(),
                evidence: Evidence::default(),
                remediation: remediation.map(str::to_string),
                method: ProbeMethod::Native,
                elapsed_ms: 1,
            });
        }
        (rules, report)
    }

    #[test]
    fn only_unhealthy_findings_produce_actions() {
        let (rules, report) = fixture();
        let plan = build_plan(&rules, &report);
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].remediation_id, "fix_a");
        assert_eq!(plan.actions[0].risk, "low");
        assert!(plan.requires_confirm());
        assert_eq!(
            plan.actions[0].rollback.as_ref().unwrap().params["enabled"],
            "true"
        );
    }

    #[test]
    fn a_failed_action_rolls_back_applied_ones() {
        let mut plan = build_plan(&fixture().0, &fixture().1);
        plan.actions.push(PlannedAction {
            remediation_id: "fix_b".to_string(),
            probe_id: "b".to_string(),
            action: "dns.set".to_string(),
            params: BTreeMap::new(),
            risk: "high".to_string(),
            needs_confirm: false,
            rollback: None,
        });
        let audit = apply_plan(
            &plan,
            &Executor {
                fail_id: Some("dns.set".to_string()),
            },
        );
        assert_eq!(audit.results.len(), 2);
        assert_eq!(audit.results[0].status, "applied");
        assert_eq!(audit.results[1].status, "failed");
        assert_eq!(audit.rollback_results.len(), 1);
        assert_eq!(audit.rollback_results[0].remediation_id, "fix_a");
        assert_eq!(audit.plan.actions.len(), 2);
    }

    #[test]
    fn a_fully_applied_plan_has_no_rollbacks() {
        let plan = build_plan(&fixture().0, &fixture().1);
        let audit = apply_plan(&plan, &Executor { fail_id: None });
        assert_eq!(audit.results[0].status, "applied");
        assert!(audit.rollback_results.is_empty());
        assert!(audit.generated_at.ends_with('Z'));
    }
}
