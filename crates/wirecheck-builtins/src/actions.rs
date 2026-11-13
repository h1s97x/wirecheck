// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Remediation executors for M2.
//!
//! The action-ID registry stays closed (see `lib.rs`). M2 deliberately ships
//! no host-mutating adapter: `DisabledExecutor` refuses every action, while
//! `RecordingExecutor` lets tests and demos exercise the whole
//! apply / rollback / audit path in memory.

use std::sync::Mutex;
use wirecheck_core::plan::{ActionExecutor, ActionOutcome, PlannedAction};

/// Safe default for `--apply`. Nothing is executed on the host until an
/// explicit platform adapter is supplied by the caller.
pub struct DisabledExecutor;

impl ActionExecutor for DisabledExecutor {
    fn apply(&self, action: &PlannedAction) -> ActionOutcome {
        blocked(action, "apply")
    }

    fn rollback(&self, action: &PlannedAction) -> ActionOutcome {
        blocked(action, "rollback")
    }
}

fn blocked(action: &PlannedAction, phase: &str) -> ActionOutcome {
    ActionOutcome {
        remediation_id: action.remediation_id.clone(),
        status: "blocked".to_string(),
        detail: format!(
            "{phase} refused: no platform adapter configured; supply an ActionExecutor"
        ),
    }
}

#[derive(Debug, Default)]
pub struct RecordingExecutor {
    pub applied: Mutex<Vec<String>>,
    pub rolled_back: Mutex<Vec<String>>,
    /// Simulate a failure for one action ID so rollback paths are testable.
    pub fail_action: Option<String>,
}

impl RecordingExecutor {
    pub fn failing(action_id: impl Into<String>) -> Self {
        Self {
            fail_action: Some(action_id.into()),
            ..Self::default()
        }
    }
}

impl ActionExecutor for RecordingExecutor {
    fn apply(&self, action: &PlannedAction) -> ActionOutcome {
        if self.fail_action.as_deref() == Some(action.action.as_str()) {
            return ActionOutcome {
                remediation_id: action.remediation_id.clone(),
                status: "failed".to_string(),
                detail: "simulated failure".to_string(),
            };
        }
        self.applied
            .lock()
            .expect("recording mutex poisoned")
            .push(action.remediation_id.clone());
        ActionOutcome {
            remediation_id: action.remediation_id.clone(),
            status: "applied".to_string(),
            detail: "recorded in memory".to_string(),
        }
    }

    fn rollback(&self, action: &PlannedAction) -> ActionOutcome {
        self.rolled_back
            .lock()
            .expect("recording mutex poisoned")
            .push(action.remediation_id.clone());
        ActionOutcome {
            remediation_id: action.remediation_id.clone(),
            status: "rolled_back".to_string(),
            detail: "recorded in memory".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use wirecheck_core::plan::{apply_plan, Plan};

    fn action(id: &str, action_id: &str) -> PlannedAction {
        PlannedAction {
            remediation_id: id.to_string(),
            probe_id: "p".to_string(),
            action: action_id.to_string(),
            params: BTreeMap::new(),
            risk: "low".to_string(),
            needs_confirm: false,
            rollback: None,
        }
    }

    fn plan() -> Plan {
        Plan {
            rule_set: "demo".to_string(),
            generated_at: "2026-11-14T00:00:00Z".to_string(),
            actions: vec![action("r1", "proxy.set"), action("r2", "dns.set")],
        }
    }

    #[test]
    fn default_executor_never_touches_the_host() {
        let audit = apply_plan(&plan(), &DisabledExecutor);
        assert!(audit
            .results
            .iter()
            .all(|result| result.status == "blocked"));
        assert!(audit.rollback_results.is_empty());
    }

    #[test]
    fn recording_executor_supports_rollback_and_audit() {
        let executor = RecordingExecutor::failing("dns.set");
        let audit = apply_plan(&plan(), &executor);
        assert_eq!(audit.results[0].status, "applied");
        assert_eq!(audit.results[1].status, "failed");
        assert_eq!(*executor.applied.lock().unwrap(), vec!["r1".to_string()]);
        assert_eq!(
            *executor.rolled_back.lock().unwrap(),
            vec!["r1".to_string()]
        );
        assert_eq!(audit.rollback_results.len(), 1);
    }
}
