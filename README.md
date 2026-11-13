# wirecheck

**Declarative network health checks, with evidence you can share.**

`wirecheck` turns "is the network broken?" into a versioned rule set: you declare
what to observe, what counts as a failure, and which remediation to suggest. The
output is a structured report you can read in a terminal, feed to CI, or attach
to a support request without leaking your local network details.

> Status: **M3 — full probe set, starter templates and rule-set trust.**
> `validate` stays offline; `run` performs network observations. Remediation
> plans are dry-run by default and no host-mutating adapter ships yet. See
> `SPEC.md` for the design and the roadmap.

## Why

Most one-click network tools hard-code the addresses of one specific
environment, produce a wall of text, and cannot be run in CI. wirecheck keeps
the environment in configuration, keeps evidence machine-readable, and treats
any fix as a dry-run plan first.

## Quick start

```bash
cargo build --workspace
cargo test --workspace

# optional: install the `wirecheck` binary into ~/.cargo/bin
cargo install --path crates/wirecheck-cli

# generate a commented starter rule set, then review and trust it
cargo run -p wirecheck-cli -- init --output wirecheck.rules.toml
cargo run -p wirecheck-cli -- validate --rules wirecheck.rules.toml
cargo run -p wirecheck-cli -- trust --rules wirecheck.rules.toml

# execute a rule set; --trust verifies its SHA-256 fingerprint
cargo run -p wirecheck-cli -- run --rules wirecheck.rules.toml --trust --stdout

# SARIF 2.1.0 for code scanning
cargo run -p wirecheck-cli -- run --rules rules/home.rules.toml --format sarif

# explicit apply: writes audit.json (no host changes without an adapter)
cargo run -p wirecheck-cli -- run --rules rules/home.rules.toml --apply --yes

wirecheck list-probes
wirecheck explain proxy.set
```

`validate` exits `0` when the rule set is valid and `2` when it is not; it
reports **all** problems in one pass. `run` exits `0` (all clear), `1` (a
`fail`/`error` finding) or `2` (invalid rule set, untrusted rule set or
configuration error). Reports are written to `[reports].output_dir`
(`--output-dir` overrides it); `--format json|html|sarif` overrides the
formats, `--stdout` also prints JSON.

M3 adds the last four probe kinds, a starter template (`init`), a local
SHA-256 trust store (`trust` / `run --trust`) and a central elevation check.
The trust store is an allow-list of reviewed fingerprints, not an asymmetric
signature scheme; `.wirecheck/` is gitignored.

The HTML output is a self-contained interactive dashboard: summary cards,
pass-rate ring, outcome filters, search, expandable redacted evidence and
dark mode. All CSS/JS is inline and it makes no network requests. It is a
**report artifact for attaching to a ticket**, not the product front end —
a GUI is explicitly out of scope in §1 of `SPEC.md`.

Safety defaults still hold: `run` prints a dry-run plan; `--apply --yes`
writes `audit.json` but `DisabledExecutor` refuses every host change, and
privileged actions are refused outright. HTTP is plaintext only and ICMP /
traceroute use system command fallbacks. See `SPEC.md` §11–§14.

## A rule set

```toml
version = 1
name = "home-network"

[defaults]
timeout = "3s"
retries = 1
concurrency = 4

[redact]
ip = "mask"
user = "hash"
mac = "mask"

[vars]
gateway = "192.0.2.1"

[[probes]]
id = "gateway_ping"
type = "icmp"
expect = { packet_loss_below = 0.5 }

[probes.with]
target = "{{ vars.gateway }}"

[[remediations]]
for = "gateway_ping"
id = "renew_interface"
action = "interface.renew"
risk = "medium"
needs_confirm = true
```

Rule-set files use the `*.rules.toml` suffix. Example addresses are
`example.com` and the RFC 5737 documentation range `192.0.2.0/24`.

## Safety model in one paragraph

A rule set is data and may come from an untrusted source. Remediation actions
are a **closed, compiled-in allow-list** referenced by ID; a rule set cannot
name an executable or a shell command. Planning is the default, `--apply` is
explicit, and M0 does not implement any apply path at all. See `SPEC.md` §5.

## Layout

| Path | Purpose |
| --- | --- |
| `crates/wirecheck-core` | schema, core types, static validator — embeddable, platform-free |
| `crates/wirecheck-builtins` | registry of probe kinds and remediation actions |
| `crates/wirecheck-report` | report types, redaction |
| `crates/wirecheck-cli` | `wirecheck` command |
| `rules/` | example rule sets |
| `SPEC.md` | normative design and record of independent authorship |

## Roadmap

- **M0** (here): governance, schema, `validate`, CI.
- **M1 / v0.1**: dns / http / icmp probes, concurrent runner, JSON + HTML reports.
- **M2 / v0.2**: dependency ordering, variables, SARIF, plan / apply / rollback.
- **M3 / v0.3**: elevation, rule-set trust, full probe set, docs.
- **v1.0**: frozen report schema, crates.io release.

## License

MIT — see `LICENSE` and `NOTICE`. Copyright (c) 2026 h1s97x.

Dependency licenses are enforced in CI with `cargo deny`. This project vendors
no third-party source code.
