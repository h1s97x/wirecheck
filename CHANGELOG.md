# Changelog

All notable changes to wirecheck are recorded here. Version tags for this
codebase start at v0.1.0; the `1.0.x` tags in the repository belong to the
legacy C# project and are not wirecheck releases.

## [Unreleased] — v1.0 preparation

- Documented report-schema stability rules and the semver commitment in
  `SPEC.md` §16.
- Added package metadata (`repository`, `readme`, `keywords`, `categories`)
  and a `wirecheck-core` embedding example.
- Reconciled the M2/M3 acceptance wording with the shipped, read-only scope
  (`SPEC.md` §15): no host-mutating adapter, no privilege escalation, and
  `--trust` is a local fingerprint allow-list rather than a signature scheme.
- Upgraded the HTML report to a self-contained interactive dashboard.
- Fixed stale milestone wording in comments and error messages.

## [0.3.0] — M3

- All eight registered probe kinds implemented.
- `wirecheck init` starter template, `wirecheck trust` / `run --trust`,
  central elevation guard.

## [0.2.0] — M2

- SARIF 2.1.0 reports, dry-run remediation plans, `--apply/--yes`, complete
  audit records and rollback orchestration.

## [0.1.0] — M1

- dns / http / icmp / tcp probes, dependency-ordered concurrent runner,
  assertion evaluation, variable substitution, UTC clock, redacted JSON/HTML
  reports.

## [0.0.1] — M0

- Governance files, rule-set schema, static validator, CLI skeleton, CI.
