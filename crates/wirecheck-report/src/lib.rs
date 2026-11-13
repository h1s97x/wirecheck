// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Redaction lives in the report layer, not in the runner: local debugging
//! keeps full values while exported reports do not.

pub mod render;

use sha2::{Digest, Sha256};
use std::fmt::Write;
use wirecheck_core::{Evidence, RedactPolicy, Redaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveKind {
    Ip,
    User,
    Mac,
    Other,
}

/// M0 classifies evidence by field-name hints. M1 replaces this with explicit
/// sensitivity tags in the rule-set schema.
pub fn classify(key: &str) -> SensitiveKind {
    let lower = key.to_ascii_lowercase();
    if lower.contains("mac") {
        SensitiveKind::Mac
    } else if lower.contains("ip") {
        SensitiveKind::Ip
    } else if lower.contains("user") || lower.contains("account") {
        SensitiveKind::User
    } else {
        SensitiveKind::Other
    }
}

pub fn redact_value(policy: &RedactPolicy, kind: SensitiveKind, value: &str) -> String {
    let mode = match kind {
        SensitiveKind::Ip => policy.ip,
        SensitiveKind::User => policy.user,
        SensitiveKind::Mac => policy.mac,
        SensitiveKind::Other => Redaction::None,
    };
    match mode {
        Redaction::None => value.to_string(),
        Redaction::Hash => hash_value(value),
        Redaction::Mask => match kind {
            SensitiveKind::Ip => mask_ip(value),
            SensitiveKind::Mac => mask_mac(value),
            _ => mask_generic(value),
        },
    }
}

pub fn redact_evidence(policy: &RedactPolicy, evidence: &Evidence) -> Evidence {
    let mut out = Evidence::default();
    for (key, value) in &evidence.fields {
        out.insert(key.clone(), redact_value(policy, classify(key), value));
    }
    out
}

pub fn mask_ip(value: &str) -> String {
    if value.contains(':') {
        let head: Vec<&str> = value.split(':').take(2).collect();
        return format!("{}:[redacted]", head.join(":"));
    }
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}.x.x", parts[0], parts[1])
    } else {
        mask_generic(value)
    }
}

pub fn mask_mac(value: &str) -> String {
    let separator = if value.contains(':') { ':' } else { '-' };
    let parts: Vec<&str> = value.split(separator).collect();
    if parts.len() == 6 {
        format!("{}:xx:xx:xx", parts[..3].join(":"))
    } else {
        mask_generic(value)
    }
}

pub fn mask_generic(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 2 {
        return "*".repeat(chars.len());
    }
    let mut out = String::new();
    out.push(chars[0]);
    out.push_str(&"*".repeat(chars.len() - 2));
    out.push(chars[chars.len() - 1]);
    out
}

pub fn hash_value(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_ipv4_and_ipv6() {
        assert_eq!(mask_ip("192.0.2.10"), "192.0.x.x");
        assert_eq!(mask_ip("2001:250:5800:11::1"), "2001:250:[redacted]");
    }

    #[test]
    fn masks_mac_and_generic_values() {
        assert_eq!(mask_mac("aa:bb:cc:dd:ee:ff"), "aa:bb:cc:xx:xx:xx");
        assert_eq!(mask_generic("abcdef"), "a****f");
    }

    #[test]
    fn hash_is_stable_and_distinct() {
        let first = hash_value("student-42");
        assert_eq!(first.len(), 64);
        assert_eq!(first, hash_value("student-42"));
        assert_ne!(first, hash_value("student-43"));
    }

    #[test]
    fn redaction_policy_is_applied_per_field() {
        let policy = RedactPolicy::default();
        let mut evidence = Evidence::default();
        evidence.insert("client_ip", "192.0.2.10");
        evidence.insert("user", "student-42");
        evidence.insert("latency_ms", "12");

        let redacted = redact_evidence(&policy, &evidence);
        assert_eq!(redacted.fields["client_ip"], "192.0.x.x");
        assert_eq!(redacted.fields["latency_ms"], "12");
        assert_eq!(redacted.fields["user"].len(), 64);
    }

    #[test]
    fn none_policy_keeps_everything() {
        let policy = RedactPolicy {
            ip: Redaction::None,
            user: Redaction::None,
            mac: Redaction::None,
        };
        assert_eq!(
            redact_value(&policy, SensitiveKind::Ip, "192.0.2.10"),
            "192.0.2.10"
        );
    }
}
