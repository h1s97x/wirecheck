// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Registry metadata for the closed set of probe kinds and remediation
//! actions. M1 implements dns / http / icmp / tcp; the other kinds stay
//! planned.
//!
//! The important property is that this registry is compiled in. A rule set can
//! only reference an ID that exists here, so an untrusted rule set can never
//! introduce a new behaviour or name an executable.

pub mod actions;
pub mod probes;

pub use actions::{DisabledExecutor, RecordingExecutor};
pub use probes::BuiltinProbes;
use wirecheck_core::{ActionKind, ACTION_IDS, PROBE_KINDS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeDescriptor {
    pub id: &'static str,
    pub summary: &'static str,
    pub implemented: bool,
    pub requires_elevation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDescriptor {
    pub id: &'static str,
    pub summary: &'static str,
    pub kind: ActionKind,
    pub requires_elevation: bool,
}

pub fn probe_descriptors() -> Vec<ProbeDescriptor> {
    PROBE_KINDS
        .iter()
        .copied()
        .map(|id| ProbeDescriptor {
            id,
            summary: probe_summary(id),
            implemented: matches!(id, "dns" | "http" | "icmp" | "tcp"),
            requires_elevation: matches!(id, "interface" | "traceroute"),
        })
        .collect()
}

pub fn action_descriptors() -> Vec<ActionDescriptor> {
    ACTION_IDS
        .iter()
        .copied()
        .map(|id| ActionDescriptor {
            id,
            summary: action_summary(id),
            kind: action_kind(id),
            requires_elevation: matches!(id, "dns.set" | "route.flush" | "interface.renew"),
        })
        .collect()
}

pub fn is_known_action(id: &str) -> bool {
    ACTION_IDS.contains(&id)
}

pub fn find_action(id: &str) -> Option<ActionDescriptor> {
    action_descriptors()
        .into_iter()
        .find(|action| action.id == id)
}

fn probe_summary(id: &str) -> &'static str {
    match id {
        "dns" => "Resolve a record against a specified resolver",
        "http" => "Issue an HTTP request and inspect status and body",
        "icmp" => "Measure reachability and packet loss (with degradation chain)",
        "tcp" => "Check whether a TCP endpoint accepts a connection",
        "proxy" => "Read the operating system proxy configuration",
        "interface" => "Inspect local interfaces, gateways and addresses",
        "traceroute" => "Trace the path towards a target",
        "captive_portal" => "Detect an interception portal",
        _ => "Planned probe kind",
    }
}

fn action_summary(id: &str) -> &'static str {
    match id {
        "proxy.set" => "Enable or disable the operating system proxy",
        "dns.set" => "Set DNS servers for an interface",
        "route.flush" => "Flush the local routing cache",
        "interface.renew" => "Renew an interface lease",
        _ => "Uncategorised action",
    }
}

fn action_kind(id: &str) -> ActionKind {
    match id {
        "route.flush" => ActionKind::NonReversible,
        _ => ActionKind::Reversible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_matches_allow_lists() {
        assert_eq!(probe_descriptors().len(), PROBE_KINDS.len());
        assert_eq!(action_descriptors().len(), ACTION_IDS.len());
    }

    #[test]
    fn unknown_actions_are_rejected() {
        assert!(is_known_action("proxy.set"));
        assert!(!is_known_action("shell.exec"));
        assert!(find_action("shell.exec").is_none());
    }

    #[test]
    fn m1_implements_four_probe_kinds() {
        let implemented: Vec<&str> = probe_descriptors()
            .iter()
            .filter(|probe| probe.implemented)
            .map(|probe| probe.id)
            .collect();
        assert_eq!(implemented, vec!["dns", "http", "icmp", "tcp"]);
        assert!(probe_descriptors().iter().any(|probe| !probe.implemented));
    }
}
