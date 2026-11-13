// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Report rendering. JSON and SARIF are machine formats; HTML is a
//! self-contained, interactive dashboard for humans. Redaction happens here,
//! never in the runner.

use crate::redact_evidence;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use wirecheck_core::{Outcome, RedactPolicy, Report};

const STYLE: &str = r#"
:root{color-scheme:light dark;--bg:#f4f6fb;--card:#fff;--text:#16202f;--muted:#66748c;--line:#e2e8f2;--pass:#0f7b4f;--fail:#c0263a;--warn:#a16207;--skip:#64748b;--accent:#3b5bdb;--shadow:0 1px 2px #0000000d}
@media (prefers-color-scheme:dark){:root{--bg:#0e1521;--card:#182131;--text:#e9eef7;--muted:#94a3b8;--line:#2b374a;--pass:#34d399;--fail:#fb7185;--warn:#fbbf24;--skip:#94a3b8;--accent:#8aa2ff;--shadow:none}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text);font:14px/1.55 system-ui,-apple-system,"Segoe UI",sans-serif}
.wrap{max-width:1120px;margin:0 auto;padding:36px 20px 64px}
header{display:flex;justify-content:space-between;align-items:flex-start;gap:16px;flex-wrap:wrap;margin-bottom:24px}
.eyebrow{color:var(--accent);font-size:11px;font-weight:700;letter-spacing:.14em;text-transform:uppercase}
h1{font-size:27px;line-height:1.2;margin:6px 0 8px;font-weight:680;letter-spacing:-.02em}
.meta{color:var(--muted);font-size:13px;margin:0}
.badge{padding:7px 14px;border-radius:999px;font-size:12px;font-weight:700;border:1px solid var(--line);background:var(--card)}
.badge.good{color:var(--pass)}.badge.warn{color:var(--warn)}.badge.bad{color:var(--fail)}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(148px,1fr));gap:12px;margin-bottom:22px}
.card{background:var(--card);border:1px solid var(--line);border-radius:14px;padding:15px 17px;box-shadow:var(--shadow)}
.label{color:var(--muted);font-size:11px;text-transform:uppercase;letter-spacing:.08em;font-weight:650}
.value{font-size:27px;font-weight:700;line-height:1.25;margin-top:5px;font-variant-numeric:tabular-nums}
.value.pass{color:var(--pass)}.value.fail{color:var(--fail)}.value.warn{color:var(--warn)}.value.skip{color:var(--skip)}
.rate{display:flex;align-items:center;justify-content:space-between;gap:10px}
.rate svg{flex:0 0 auto}
.panel{background:var(--card);border:1px solid var(--line);border-radius:14px;box-shadow:var(--shadow);overflow:hidden}
.toolbar{display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap;padding:14px 16px;border-bottom:1px solid var(--line)}
.filters{display:flex;gap:6px;flex-wrap:wrap}
button[data-filter]{border:1px solid var(--line);background:transparent;color:var(--muted);padding:6px 12px;border-radius:999px;font:inherit;font-size:12px;font-weight:650;cursor:pointer}
button[data-filter] span{opacity:.7;margin-left:4px}
button[data-filter].active{background:var(--accent);border-color:var(--accent);color:#fff}
input[type=search]{border:1px solid var(--line);background:var(--bg);color:var(--text);border-radius:9px;padding:8px 12px;font:inherit;font-size:13px;min-width:220px}
input[type=search]:focus,button[data-filter]:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.table-scroll{overflow-x:auto}
table{width:100%;border-collapse:collapse;font-size:13px}
th{text-align:left;color:var(--muted);font-size:11px;text-transform:uppercase;letter-spacing:.07em;padding:12px 16px;background:var(--bg);border-bottom:1px solid var(--line);white-space:nowrap}
td{padding:13px 16px;border-bottom:1px solid var(--line);vertical-align:top}
tbody tr:last-child td{border-bottom:0}
tbody tr:hover{background:var(--bg)}
code,.mono{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:12px}
.msg{max-width:380px}
.pill{display:inline-flex;align-items:center;gap:6px;padding:3px 10px;border-radius:999px;font-size:11px;font-weight:700;text-transform:capitalize;white-space:nowrap}
.dot{width:6px;height:6px;border-radius:50%;background:currentColor}
.pill.pass{color:var(--pass);background:#0f7b4f1a}.pill.fail,.pill.error{color:var(--fail);background:#c0263a1a}
.pill.warn{color:var(--warn);background:#a162071a}.pill.skipped{color:var(--skip);background:#64748b1a}
details summary{cursor:pointer;color:var(--accent);font-size:12px}
.kv{margin-top:8px;display:grid;gap:4px}
.kv div{display:flex;gap:8px;font-size:12px}
.kv b{font-weight:600;color:var(--muted);min-width:90px;word-break:break-all}
.kv span{word-break:break-all}
.muted{color:var(--muted)}
.empty{padding:28px 16px;text-align:center;color:var(--muted);margin:0}
footer{color:var(--muted);font-size:12px;margin-top:20px;text-align:center}
@media (max-width:640px){.wrap{padding:22px 12px 48px}input[type=search]{min-width:0;width:100%}}
"#;

const SCRIPT: &str = r#"
(function () {
  var rows = Array.prototype.slice.call(document.querySelectorAll('tbody tr'));
  var buttons = Array.prototype.slice.call(document.querySelectorAll('[data-filter]'));
  var search = document.getElementById('q');
  var empty = document.getElementById('empty');
  var filter = 'all';
  function apply() {
    var term = search.value.trim().toLowerCase();
    var visible = 0;
    rows.forEach(function (row) {
      var outcome = row.getAttribute('data-outcome');
      var matchesFilter = filter === 'all'
        || (filter === 'issue' && (outcome === 'fail' || outcome === 'error'))
        || (filter === 'warn' && outcome === 'warn')
        || (filter === 'pass' && (outcome === 'pass' || outcome === 'skipped'));
      var matchesText = !term || row.getAttribute('data-search').indexOf(term) !== -1;
      var show = matchesFilter && matchesText;
      row.hidden = !show;
      if (show) { visible += 1; }
    });
    empty.hidden = visible !== 0;
  }
  buttons.forEach(function (button) {
    button.addEventListener('click', function () {
      filter = button.getAttribute('data-filter');
      buttons.forEach(function (other) { other.classList.toggle('active', other === button); });
      apply();
    });
  });
  search.addEventListener('input', apply);
  apply();
})();
"#;

pub fn render_json(report: &Report, policy: &RedactPolicy) -> Result<String, serde_json::Error> {
    let mut exported = report.clone();
    for finding in &mut exported.findings {
        finding.evidence = redact_evidence(policy, &finding.evidence);
    }
    serde_json::to_string_pretty(&exported)
}

/// A single, offline, interactive dashboard. All CSS and JS are inline; no
/// fonts, images, scripts or network requests are loaded from anywhere.
pub fn render_html(report: &Report, policy: &RedactPolicy) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in &report.findings {
        *counts.entry(outcome_key(finding.outcome)).or_insert(0) += 1;
    }
    let count = |key: &str| *counts.get(key).unwrap_or(&0);
    let total = report.findings.len();
    let pass = count("pass");
    let fail = count("fail");
    let error = count("error");
    let warn = count("warn");
    let skipped = count("skipped");
    let issues = fail + error;
    let healthy = pass + skipped;
    let rate = (pass * 100).checked_div(total).unwrap_or(100);
    let (badge_class, badge_text) = if issues > 0 {
        ("bad", "Issues found")
    } else if warn > 0 {
        ("warn", "Warnings")
    } else {
        ("good", "All clear")
    };

    let card = |label: &str, value: usize, class: &str| {
        format!(
            "<div class=\"card\"><div class=\"label\">{label}</div><div class=\"value {class}\">{value}</div></div>"
        )
    };
    let donut = format!(
        "<div class=\"card rate\"><div><div class=\"label\">Pass rate</div><div class=\"value\">{rate}%</div></div><svg viewBox=\"0 0 42 42\" width=\"66\" height=\"66\" role=\"img\" aria-label=\"{rate}% passed\"><circle cx=\"21\" cy=\"21\" r=\"16\" fill=\"none\" stroke=\"var(--line)\" stroke-width=\"5\"/><circle cx=\"21\" cy=\"21\" r=\"16\" fill=\"none\" stroke=\"var(--pass)\" stroke-width=\"5\" stroke-linecap=\"round\" pathLength=\"100\" stroke-dasharray=\"{rate} 100\" transform=\"rotate(-90 21 21)\"/></svg></div>"
    );

    let mut output = String::new();
    output.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    output.push_str(&format!(
        "<title>{} — wirecheck</title>\n",
        escape(&report.rule_set)
    ));
    output.push_str("<style>\n");
    output.push_str(STYLE);
    output.push_str("\n</style>\n</head>\n<body>\n<div class=\"wrap\">\n");

    output.push_str(&format!(
        "<header><div><div class=\"eyebrow\">wirecheck · run report</div><h1>{}</h1><p class=\"meta\">UTC {} · report schema {} · {} probes · exit code {}</p></div><span class=\"badge {}\">{}</span></header>\n",
        escape(&report.rule_set),
        escape(&report.generated_at),
        report.schema_version,
        total,
        report.exit_code(),
        badge_class,
        badge_text,
    ));

    output.push_str("<section class=\"cards\" aria-label=\"Summary\">\n");
    output.push_str(&card("Probes", total, ""));
    output.push_str(&card("Healthy", healthy, "pass"));
    output.push_str(&card("Issues", issues, "fail"));
    output.push_str(&card("Warnings", warn, "warn"));
    output.push_str(&card("Skipped", skipped, "skip"));
    output.push_str(&donut);
    output.push_str("\n</section>\n");

    output.push_str("<section class=\"panel\">\n<div class=\"toolbar\"><div class=\"filters\" role=\"group\" aria-label=\"Filter results\">\n");
    for (key, label, value) in [
        ("all", "All", total),
        ("issue", "Issues", issues),
        ("warn", "Warnings", warn),
        ("pass", "Healthy", healthy),
    ] {
        let active = if key == "all" { " active" } else { "" };
        output.push_str(&format!(
            "<button type=\"button\" data-filter=\"{key}\" class=\"{trimmed}\">{label} <span>{value}</span></button>\n",
            trimmed = active.trim(),
        ));
    }
    output.push_str("</div><input id=\"q\" type=\"search\" placeholder=\"Search probes…\" aria-label=\"Search probes\" autocomplete=\"off\">\n</div>\n");
    output.push_str("<div class=\"table-scroll\"><table>\n<thead><tr><th>Probe</th><th>Status</th><th>Message</th><th>Evidence</th><th>Method</th><th>Time</th></tr></thead>\n<tbody>\n");

    for finding in &report.findings {
        let key = outcome_key(finding.outcome);
        let evidence = redact_evidence(policy, &finding.evidence);
        let mut details = String::new();
        for (field, value) in &evidence.fields {
            details.push_str(&format!(
                "<div><b>{}</b><span>{}</span></div>",
                escape(field),
                escape(value)
            ));
        }
        let evidence_cell = if evidence.fields.is_empty() {
            "<span class=\"muted\">—</span>".to_string()
        } else {
            format!(
                "<details><summary>{} fields</summary><div class=\"kv\">{}</div></details>",
                evidence.fields.len(),
                details
            )
        };
        let search =
            escape(&format!("{} {}", finding.probe_id, finding.message).to_ascii_lowercase());
        output.push_str(&format!(
            "<tr data-outcome=\"{key}\" data-search=\"{search}\"><td><code>{probe}</code></td><td><span class=\"pill {key}\"><i class=\"dot\"></i>{key}</span></td><td class=\"msg\">{message}</td><td>{evidence_cell}</td><td class=\"mono\">{method:?}</td><td class=\"mono\">{elapsed} ms</td></tr>\n",
            probe = escape(&finding.probe_id),
            message = escape(&finding.message),
            method = finding.method,
            elapsed = finding.elapsed_ms,
        ));
    }

    output.push_str("</tbody>\n</table>\n<p id=\"empty\" class=\"empty\" hidden>No findings match this filter.</p>\n</div>\n</section>\n");
    output.push_str("<footer>Self-contained report · inline CSS and JS · no network requests · evidence redacted at render time</footer>\n</div>\n<script>\n");
    output.push_str(SCRIPT);
    output.push_str("\n</script>\n</body>\n</html>\n");
    output
}

fn outcome_key(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Pass => "pass",
        Outcome::Fail => "fail",
        Outcome::Warn => "warn",
        Outcome::Skipped => "skipped",
        Outcome::Error => "error",
    }
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

/// SARIF 2.1.0 for code-scanning surfaces. Evidence is redacted here too.
pub fn render_sarif(report: &Report, policy: &RedactPolicy) -> Result<String, serde_json::Error> {
    let mut results = Vec::new();
    for finding in &report.findings {
        let evidence = redact_evidence(policy, &finding.evidence);
        results.push(serde_json::json!({
            "ruleId": finding.probe_id,
            "level": sarif_level(finding.outcome),
            "message": { "text": finding.message },
            "properties": {
                "outcome": format!("{:?}", finding.outcome).to_ascii_lowercase(),
                "evidence": evidence.fields,
            }
        }));
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": { "name": "wirecheck", "version": "0.1.0" } },
            "results": results,
        }]
    }))
}

fn sarif_level(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Fail | Outcome::Error => "error",
        Outcome::Warn => "warning",
        Outcome::Pass | Outcome::Skipped => "none",
    }
}

/// Write the requested formats. Unknown formats are ignored rather than
/// producing a misleading empty file.
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
            "sarif" => (
                "report.sarif",
                render_sarif(report, policy)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            ),
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
    use wirecheck_core::{Evidence, Finding, ProbeMethod};

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
        assert!(html.contains("<script>"));
        assert!(!html.contains("<script src"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("<link"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("192.0.x.x"));
        assert!(!html.contains("192.0.2.10"));
        assert!(html.contains("prefers-color-scheme"));
    }

    #[test]
    fn html_has_dashboard_controls_and_summary() {
        let html = render_html(&sample(), &RedactPolicy::default());
        for marker in [
            "data-filter=\"all\"",
            "data-filter=\"issue\"",
            "id=\"q\"",
            "Pass rate",
            "Issues found",
            "exit code 1",
            "<svg",
            "aria-label",
        ] {
            assert!(html.contains(marker), "missing dashboard marker: {marker}");
        }
    }

    #[test]
    fn an_empty_report_is_all_clear() {
        let report = Report::new("empty", "2026-11-14T00:00:00Z");
        let html = render_html(&report, &RedactPolicy::default());
        assert!(html.contains("All clear"));
        assert!(html.contains("100%"));
        assert!(html.contains("No findings match this filter."));
    }

    #[test]
    fn sarif_is_valid_and_redacted() {
        let text = render_sarif(&sample(), &RedactPolicy::default()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["version"], "2.1.0");
        assert_eq!(value["runs"][0]["results"][0]["level"], "error");
        assert_eq!(value["runs"][0]["results"][0]["ruleId"], "p");
        assert!(text.contains("192.0.x.x"));
        assert!(!text.contains("192.0.2.10"));
    }

    #[test]
    fn all_report_formats_are_written() {
        let directory =
            std::env::temp_dir().join(format!("wirecheck-render-{}-{}", std::process::id(), "m2"));
        let written = write_reports(
            &sample(),
            &RedactPolicy::default(),
            &["sarif".to_string(), "json".to_string(), "html".to_string()],
            &directory,
        )
        .unwrap();
        assert_eq!(written.len(), 3);
        assert!(written.iter().any(|path| path.ends_with("report.sarif")));
        let _ = fs::remove_dir_all(&directory);
    }
}
