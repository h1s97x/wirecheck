// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Report rendering for M1: JSON for programs and a self-contained HTML file
//! for humans. Redaction happens here, never in the runner.

use crate::redact_evidence;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use wirecheck_core::{RedactPolicy, Report};

const STYLE: &str = "body{margin:0;background:#f6f7f9;color:#1c2330;font:15px/1.5 system-ui,sans-serif}main{max-width:960px;margin:40px auto;padding:0 20px}h1{font-size:24px}.meta{color:#5d6878}table{width:100%;border-collapse:collapse;background:#fff;box-shadow:0 1px 3px #0001}th,td{padding:10px;border-bottom:1px solid #e6e9ee;text-align:left;vertical-align:top}th{background:#eef1f6}code{font-size:13px}ul{margin:0;padding-left:18px}.pass{color:#176b3a}.fail,.error{color:#a1202c}.warn{color:#8a5a00}.skipped{color:#5d6878}";

pub fn render_json(report: &Report, policy: &RedactPolicy) -> Result<String, serde_json::Error> {
    let mut exported = report.clone();
    for finding in &mut exported.findings {
        finding.evidence = redact_evidence(policy, &finding.evidence);
    }
    serde_json::to_string_pretty(&exported)
}

pub fn render_html(report: &Report, policy: &RedactPolicy) -> String {
    let mut output = String::new();
    output.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    output.push_str(&format!(
        "<title>{} — wirecheck report</title>\n",
        escape(&report.rule_set)
    ));
    output.push_str("<style>\n");
    output.push_str(STYLE);
    output.push_str("\n</style>\n</head>\n<body>\n<main>\n");
    output.push_str(&format!("<h1>{}</h1>\n", escape(&report.rule_set)));
    output.push_str(&format!(
        "<p class=\"meta\">report schema {} · generated {} · {} findings</p>\n",
        report.schema_version,
        escape(&report.generated_at),
        report.findings.len()
    ));
    output.push_str("<table>\n<thead><tr><th>Probe</th><th>Outcome</th><th>Message</th><th>Evidence</th><th>Method</th><th>ms</th></tr></thead>\n<tbody>\n");

    for finding in &report.findings {
        let evidence = redact_evidence(policy, &finding.evidence);
        let mut details = String::new();
        for (key, value) in &evidence.fields {
            details.push_str(&format!(
                "<li><code>{}</code>: {}</li>",
                escape(key),
                escape(value)
            ));
        }
        let outcome = format!("{:?}", finding.outcome);
        output.push_str(&format!(
            "<tr class=\"{class}\"><td>{probe}</td><td>{outcome}</td><td>{message}</td><td><ul>{details}</ul></td><td>{method:?}</td><td>{elapsed}</td></tr>\n",
            class = escape(&outcome.to_ascii_lowercase()),
            probe = escape(&finding.probe_id),
            outcome = escape(&outcome),
            message = escape(&finding.message),
            details = details,
            method = finding.method,
            elapsed = finding.elapsed_ms,
        ));
    }

    output.push_str("</tbody>\n</table>\n</main>\n</body>\n</html>\n");
    output
}

fn escape(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            other => output.push(other),
        }
    }
    output
}

/// Write the requested formats. SARIF is an M2 deliverable and is ignored in
/// M1 rather than producing a misleading empty file.
pub fn write_reports(
    report: &Report,
    policy: &RedactPolicy,
    formats: &[String],
    output_dir: &Path,
) -> io::Result<Vec<PathBuf>> {
    fs::create_dir_all(output_dir)?;
    let mut written = Vec::new();
    for format in formats {
        let (filename, content) = match format.as_str() {
            "json" => (
                "report.json",
                render_json(report, policy)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            ),
            "html" => ("report.html", render_html(report, policy)),
            _ => continue,
        };
        let path = output_dir.join(filename);
        fs::write(&path, content)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wirecheck_core::{Evidence, Finding, Outcome, ProbeMethod};

    fn sample() -> Report {
        let mut report = Report::new("demo", "2026-11-14T00:00:00Z");
        let mut evidence = Evidence::default();
        evidence.insert("client_ip", "192.0.2.10");
        evidence.insert("payload", "<script>alert(1)</script>");
        report.findings.push(Finding {
            probe_id: "p".to_string(),
            outcome: Outcome::Fail,
            message: "<script>alert(1)</script>".to_string(),
            evidence,
            remediation: None,
            method: ProbeMethod::Native,
            elapsed_ms: 12,
        });
        report
    }

    #[test]
    fn json_is_redacted_and_parseable() {
        let json = render_json(&sample(), &RedactPolicy::default()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["schema_version"].as_u64(), Some(1));
        assert!(json.contains("192.0.x.x"));
        assert!(!json.contains("192.0.2.10"));
    }

    #[test]
    fn html_is_self_contained_and_escaped() {
        let html = render_html(&sample(), &RedactPolicy::default());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<style>"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("192.0.x.x"));
        assert!(!html.contains("192.0.2.10"));
        assert!(!html.contains("http://"));
    }

    #[test]
    fn sarif_is_not_written_in_m1() {
        let directory =
            std::env::temp_dir().join(format!("wirecheck-render-{}-{}", std::process::id(), "m1"));
        let written = write_reports(
            &sample(),
            &RedactPolicy::default(),
            &["sarif".to_string(), "json".to_string()],
            &directory,
        )
        .unwrap();
        assert_eq!(written.len(), 1);
        assert!(written[0].ends_with("report.json"));
        let _ = fs::remove_dir_all(&directory);
    }
}
