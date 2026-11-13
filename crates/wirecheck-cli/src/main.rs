// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use wirecheck_builtins::{
    action_descriptors, find_action, probe_descriptors, BuiltinProbes, DisabledExecutor,
};
use wirecheck_core::plan::{apply_plan, build_plan};
use wirecheck_core::{validate, Outcome, RuleSet, Severity};
use wirecheck_report::render::{render_json, write_reports};

#[derive(Debug, Parser)]
#[command(
    name = "wirecheck",
    version,
    about = "Declarative network health checks, with evidence you can share",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Statically validate a rule set. Never touches the network.
    Validate {
        /// Path to a *.rules.toml file
        #[arg(short, long, value_name = "FILE")]
        rules: PathBuf,
        /// Emit diagnostics as JSON, for CI consumption
        #[arg(long)]
        json: bool,
    },
    /// Execute a rule set and write reports (read-only unless --apply is given)
    Run {
        /// Path to a *.rules.toml file
        #[arg(short, long, value_name = "FILE")]
        rules: PathBuf,
        /// Override [reports].output_dir
        #[arg(long, value_name = "DIR")]
        output_dir: Option<PathBuf>,
        /// Override [reports].formats (repeatable: json, html, sarif)
        #[arg(long = "format", value_name = "FORMAT")]
        formats: Vec<String>,
        /// Also print the JSON report to stdout
        #[arg(long)]
        stdout: bool,
        /// Apply the remediation plan (requires --yes)
        #[arg(long)]
        apply: bool,
        /// Confirm --apply non-interactively
        #[arg(long)]
        yes: bool,
    },
    /// List built-in probe kinds and their implementation status
    ListProbes,
    /// Show metadata for a remediation action ID
    Explain {
        #[arg(value_name = "ACTION_ID")]
        action: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            2
        }
    };
    std::process::exit(code);
}

fn run(cli: Cli) -> Result<i32> {
    match cli.command {
        Command::Validate { rules, json } => validate_command(&rules, json),
        Command::Run {
            rules,
            output_dir,
            formats,
            stdout,
            apply,
            yes,
        } => run_command(&rules, output_dir.as_deref(), &formats, stdout, apply, yes),
        Command::ListProbes => {
            let implemented = probe_descriptors()
                .iter()
                .filter(|probe| probe.implemented)
                .count();
            println!("probe kinds (M1: {implemented} implemented)");
            for probe in probe_descriptors() {
                let status = if probe.implemented {
                    "implemented"
                } else {
                    "planned"
                };
                let elevated = if probe.requires_elevation {
                    ", elevation"
                } else {
                    ""
                };
                println!(
                    "  {:<16} [{}{}] {}",
                    probe.id, status, elevated, probe.summary
                );
            }
            Ok(0)
        }
        Command::Explain { action } => match find_action(&action) {
            Some(descriptor) => {
                println!("action    : {}", descriptor.id);
                println!("summary   : {}", descriptor.summary);
                println!("kind      : {:?}", descriptor.kind);
                println!("elevation : {}", descriptor.requires_elevation);
                Ok(0)
            }
            None => {
                eprintln!("unknown action id `{action}`");
                let known: Vec<&str> = action_descriptors().iter().map(|a| a.id).collect();
                eprintln!("known ids : {}", known.join(", "));
                Ok(2)
            }
        },
    }
}

fn run_command(
    path: &Path,
    output_dir: Option<&Path>,
    overrides: &[String],
    to_stdout: bool,
    apply: bool,
    yes: bool,
) -> Result<i32> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read rule set `{}`", path.display()))?;
    let rules = RuleSet::from_toml(&text).context("invalid rule set")?;
    let validation = validate(&rules);
    if !validation.is_ok() {
        // Reuse the existing printer so all diagnostics are shown in one pass.
        return validate_command(path, false);
    }

    let report = wirecheck_core::runner::run(&rules, &BuiltinProbes);
    let formats: Vec<String> = if overrides.is_empty() {
        rules.reports.formats.clone()
    } else {
        overrides.to_vec()
    };
    let directory = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&rules.reports.output_dir));
    let written = write_reports(&report, &rules.redact, &formats, &directory)?;

    if to_stdout {
        println!("{}", render_json(&report, &rules.redact)?);
    }
    println!("rule set : {}", report.rule_set);
    println!("clock    : {}", report.generated_at);
    for finding in &report.findings {
        println!(
            "[{}] {}: {}",
            outcome_label(finding.outcome),
            finding.probe_id,
            finding.message
        );
    }
    for file in &written {
        println!("report   : {}", file.display());
    }

    let plan = build_plan(&rules, &report);
    if apply {
        if !yes {
            bail!("--apply requires --yes; plans are dry-run by default");
        }
        if plan.requires_confirm() && !std::io::stdin().is_terminal() {
            bail!("a remediation has needs_confirm = true; an interactive TTY is required");
        }
        fs::create_dir_all(&directory)?;
        // M2 ships no host-mutating adapter. Every action is refused and the
        // refusal is recorded, so an apply attempt is still fully auditable.
        let audit = apply_plan(&plan, &DisabledExecutor);
        let audit_path = directory.join("audit.json");
        fs::write(&audit_path, serde_json::to_string_pretty(&audit)?)?;
        for result in &audit.results {
            println!(
                "[{}] {}: {}",
                result.status, result.remediation_id, result.detail
            );
        }
        println!("audit    : {}", audit_path.display());
    } else if plan.actions.is_empty() {
        println!("plan     : no remediation suggested (dry-run)");
    } else {
        println!(
            "plan     : {} action(s), dry-run, nothing applied",
            plan.actions.len()
        );
        for action in &plan.actions {
            println!(
                "  [{}] {}: {} needs_confirm={}",
                action.risk, action.remediation_id, action.action, action.needs_confirm
            );
        }
    }
    Ok(report.exit_code())
}

fn outcome_label(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Pass => "pass",
        Outcome::Fail => "fail",
        Outcome::Warn => "warn",
        Outcome::Skipped => "skipped",
        Outcome::Error => "error",
    }
}

fn validate_command(path: &Path, json: bool) -> Result<i32> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read rule set `{}`", path.display()))?;
    let rules = RuleSet::from_toml(&text).context("invalid rule set")?;
    let report = validate(&rules);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(report.exit_code());
    }

    println!("rule set : {}", report.rule_set);
    println!("probes   : {}", report.probe_count);
    println!("remedies : {}", report.remediation_count);
    println!();
    if report.diagnostics.is_empty() {
        println!("OK: no diagnostics.");
    } else {
        for diagnostic in &report.diagnostics {
            println!(
                "[{}] {} {} :: {}",
                severity_label(diagnostic.severity),
                diagnostic.code,
                diagnostic.path,
                diagnostic.message
            );
        }
        println!();
        println!(
            "{} error(s), {} warning(s)",
            report.error_count(),
            report.warning_count()
        );
    }
    println!(
        "validation {}",
        if report.is_ok() { "passed" } else { "failed" }
    );
    Ok(report.exit_code())
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}
