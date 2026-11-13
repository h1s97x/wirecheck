// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Minimal embedding example: parse and statically validate a rule set using
//! only `wirecheck-core`, with no CLI, probe execution or network access.

use wirecheck_core::{validate, RuleSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = r#"
version = 1
name = "embedded"
[[probes]]
id = "loopback"
type = "tcp"
expect = { status = "refused" }
[probes.with]
host = "127.0.0.1"
port = 1
"#;
    let rules = RuleSet::from_toml(text)?;
    let report = validate(&rules);
    println!("rule set    : {}", report.rule_set);
    println!("probes      : {}", report.probe_count);
    println!("diagnostics : {}", report.diagnostics.len());
    Ok(())
}
