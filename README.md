<div align="center">
  <img src="public/nexus.svg" alt="NEXUS" width="82" />

# NEXUS

**Offensive Security Engagement Platform**

_Map assets. Track access. Understand the path._

[![Release](https://img.shields.io/badge/release-v1.1.1-f2f3f5?style=flat-square&labelColor=111418)](https://github.com/TW4RDYDEV/NEXUS/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64%20%7C%20Linux%20x86__64-f2f3f5?style=flat-square&labelColor=111418)](#installation)
[![Tauri](https://img.shields.io/badge/Tauri-2-f2f3f5?style=flat-square&labelColor=111418)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/core-Rust-f2f3f5?style=flat-square&labelColor=111418)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-source--available-f2f3f5?style=flat-square&labelColor=111418)](LICENSE)

**by TWARDY.exe / TW4RDYDEV**

</div>

---

NEXUS is a local-first offensive-security workspace that turns scattered assessment data into one connected operational view. It correlates assets, services, credentials, sessions, pivots, findings, evidence, coverage, and historical changes so an operator can understand not only **what exists**, but **what is reachable, what access is confirmed, what has been tested, and what still needs attention**.

NEXUS does not try to replace Nmap, Nuclei, httpx, NetExec, Burp, or other established tools. It acts as the engagement layer above them.

```text
TOOLS → NORMALIZED STATE → RELATIONSHIPS → COVERAGE → ATTACK PATH → EVIDENCE
```

## Preview

### Operational overview

![NEXUS overview](docs/screenshots/overview.png)

### Asset graph

![NEXUS graph](docs/screenshots/graph.png)

<details>
<summary><strong>More screenshots</strong></summary>

### Credential access matrix

![NEXUS credentials](docs/screenshots/credentials.png)

### Pivots & routes

![NEXUS pivots](docs/screenshots/pivots.png)

### ReconDelta snapshots

![NEXUS snapshots](docs/screenshots/snapshots.png)

### Assessment coverage

![NEXUS coverage](docs/screenshots/coverage.png)

### Assessment planning

![NEXUS assessment](docs/screenshots/assessment.png)

</details>

## Why NEXUS

A real engagement quickly becomes more than a folder of scan outputs. Hosts appear under different names, credentials work on some services but not others, pivots change network reachability, scanner findings need evidence, and repeated recon changes the known attack surface.

NEXUS keeps that state connected and explainable.

- **One engagement model** for assets, identities, access, findings, evidence, and scope.
- **Interactive relationship graph** instead of disconnected scanner output.
- **Encrypted credential vault** with a host/service access matrix.
- **Sessions and pivots** modeled as first-class operational state.
- **Deterministic coverage and opportunities** based only on recorded facts.
- **ReconDelta** snapshot comparison for new, changed, and removed state.
- **Data provenance** so important observations retain their source.
- **Local-first architecture** with no account, cloud backend, telemetry, analytics, or LLM dependency.

## Core capabilities

### Engagement intelligence

- Separate assessment workspaces with explicit include/exclude scope rules.
- Hosts, networks, domains, users, web applications, services, aliases, tags, and analyst notes.
- Field-level provenance and confidence-aware observation handling.
- Fast search, filtering, pagination, and keyboard-first navigation.

### Graph & attack path

- Cytoscape-based operational graph.
- Assets, services, identities, credentials, sessions, pivots, findings, and relationships.
- Subnet grouping, focus mode, neighborhood filtering, and graph layers.
- Confirmed-access path highlighting based on recorded state rather than speculative AI output.

### Credentials & access

- Argon2id-derived vault key.
- XChaCha20-Poly1305 authenticated encryption.
- Secrets masked by default with deliberate temporary reveal.
- Credential → host → service authentication history.
- Valid / invalid / untested access matrix.
- Active sessions, privilege state, and confirmed-access relationships.

### Pivots & reachability

- Ligolo, SOCKS, SSH tunnel, port-forward, and generic pivot modeling.
- Session-backed routes.
- Recursive network reachability calculation.
- Explicit `reachable via` context instead of treating discovery as access.

### Findings & evidence

- Finding status, severity, affected asset/service, impact, remediation, CWE/CVE/CVSS metadata.
- Text, screenshot, terminal output, HTTP, and file evidence.
- Evidence integrity verification.
- Markdown and client-safe HTML report export.

### Assessment & coverage

- Built-in assessment planning with original methodology checks across multiple domains.
- Custom checks, ownership, deadlines, outcomes, evidence, and finding links.
- Coverage states: Complete, Partial, Untested, Not Applicable.
- Explainable opportunity engine for missing recorded work.

### Snapshots / ReconDelta

- Named engagement snapshots.
- Added / changed / removed comparisons.
- Field-level diff inspection.
- Graph overlays for meaningful changes between snapshots.

### Recovery & portability

- SQLite WAL-backed local storage.
- Transactional imports.
- Automatic and pre-import backups.
- Recovery into a separate workspace.
- Complete workspace bundle export with attachment manifests and integrity verification.

## Supported imports

| Tool / format       | Support                         |
| ------------------- | ------------------------------- |
| Nmap XML            | Import + reviewed normalization |
| httpx JSON / JSONL  | Import + service enrichment     |
| Nuclei JSONL        | Import + draft findings         |
| NetExec output      | Import + access observations    |
| Nessus v2 `.nessus` | Import                          |
| Burp Issues XML     | Import                          |

NEXUS also detects local installations of Nmap, httpx, Nuclei, and NetExec. The built-in active runner is intentionally limited to a fixed, scope-validated Nmap workflow.

## Typical workflow

```text
Create engagement
      ↓
Define authorized scope
      ↓
Import / run reconnaissance
      ↓
Review assets & services
      ↓
Record credentials and authentication results
      ↓
Create sessions
      ↓
Model pivots and new reachability
      ↓
Track findings and evidence
      ↓
Review coverage and untested opportunities
      ↓
Capture snapshots / compare changes
      ↓
Export report or verified workspace bundle
```

## Installation

NEXUS supports **Windows x64** and **Linux x86_64**.

Download the latest release from:

**[GitHub Releases](https://github.com/TW4RDYDEV/NEXUS/releases/latest)**

Release artifacts include SHA-256 checksums for verification.

### Windows

Available packages:

- Windows installer
- Standalone executable
- Portable ZIP

> Community builds may be unsigned. Windows can therefore display an Unknown Publisher / SmartScreen warning until code signing is introduced.

### Linux

Available packages:

- **AppImage** — recommended for Arch Linux and other distributions
- **DEB** — Debian / Ubuntu
- **RPM** — Fedora / RHEL-compatible distributions

#### AppImage

```bash
chmod +x NEXUS-1.1.1-x86_64.AppImage
./NEXUS-1.1.1-x86_64.AppImage
```

NEXUS v1.1.1 includes an AppImage compatibility fix for newer Mesa/EGL/Wayland environments, including current Arch Linux systems.

#### Debian / Ubuntu

```bash
sudo apt install ./NEXUS-1.1.1-amd64.deb
```

#### Fedora / RHEL

```bash
sudo dnf install ./NEXUS-1.1.1-x86_64.rpm
```

Nmap and optional external integrations such as httpx, Nuclei, and NetExec must be installed separately.

### Run from source

Common requirements:

- Node.js 24.x
- Rust stable

Windows additionally requires:

- Microsoft C++ Build Tools + Windows SDK
- Microsoft Edge WebView2 Runtime

Linux requires the native GTK/WebKit dependencies used by Tauri.

```bash
git clone https://github.com/TW4RDYDEV/NEXUS.git
cd NEXUS
npm ci
npm run desktop
```

This launches the real Tauri desktop application backed by the Rust core and SQLite database.

## Development & verification

```bash
npm run format:check
npm run lint
npm run typecheck
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:e2e
npm run build
npm run fingerprint:test
npm run authorship:check
```

### Windows release build

Run on Windows:

```powershell
npm ci
npm run release:check
npm run release:build
```

The Windows release builder remaps local Rust/Cargo paths, stages the Windows x64 artifacts, checks embedded NEXUS identity markers, rejects known local build-path leakage, and creates SHA-256 sums.

### Linux release build

Run on Linux x86_64:

```bash
npm ci
npm run release:check
npm run release:linux
```

The Linux release builder produces AppImage, DEB, and RPM packages, performs release validation and deterministic artifact staging, sanitizes the AppImage against known Mesa/EGL/Wayland display-stack conflicts, verifies the final AppImage library policy, and generates SHA-256 checksums.

The repository also includes a GitHub Actions Linux build workflow using Ubuntu 22.04. The workflow builds the Linux release packages, validates the sanitized AppImage, and performs an automated AppImage launch smoke test before artifacts are uploaded.

See [Verification](docs/VERIFICATION.md), [Architecture](docs/ARCHITECTURE.md), and [Security](SECURITY.md) for more detail.

## Architecture

```text
React + TypeScript / Cytoscape
          │
          │ validated Tauri IPC
          ▼
      Rust core
          │
          ├── SQLite + migrations
          ├── parser adapters
          ├── encrypted vault
          ├── graph / paths
          ├── reachability
          ├── coverage
          ├── snapshots
          ├── evidence / backups
          └── scoped Nmap runner
```

NEXUS keeps core domain rules in Rust and keeps the React renderer focused on presentation and interaction.

## Privacy & security model

NEXUS is local-first.

- No user account.
- No cloud synchronization.
- No telemetry or analytics.
- No hidden network callbacks.
- No LLM dependency.
- Credential secrets are encrypted at rest.
- Imported files are treated as untrusted input.
- Attachments are never automatically executed.
- Active Nmap execution is explicitly initiated and checked against engagement scope.

The rest of the engagement database is **not** fully encrypted, so hostnames, findings, notes, and evidence should be treated as sensitive assessment data. Full-disk encryption and proper filesystem permissions are strongly recommended.

Read the complete [security model](SECURITY.md).

## Responsible use

NEXUS is intended for authorized penetration testing, security research, CTFs, labs, and defensive assessment workflows.

**Only assess systems and networks that you own or have explicit permission to test.**

The presence of a target in NEXUS does not constitute authorization.

## License

NEXUS is **source-available**, not OSI open-source software.

Under the [NEXUS Source-Available License 1.0](LICENSE), you may use NEXUS free of charge, inspect the source, and make private modifications for your own or your organization's internal use.

Professional security use — including paid authorized assessments — is permitted. What is not permitted without prior written authorization is redistributing or mirroring copies of NEXUS, selling it, publishing modified builds, rebranding it, sublicensing it, or offering NEXUS itself as a paid/hosted product.

A limited public-fork exception exists for good-faith contributions to the official project. See [LICENSE](LICENSE) for the complete terms.

## Contributing

Bug reports, documentation improvements, parser fixes, tests, and carefully scoped feature contributions are welcome.

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting changes.

## Support the project

If NEXUS is useful to you, the simplest ways to support development are:

- star the repository;
- report reproducible bugs;
- improve documentation or tests;
- submit focused pull requests;
- sponsor development through the repository Sponsor button when available.

## Author

**TWARDY.exe / TW4RDYDEV**

Cybersecurity · Offensive Security · Networking · Software Development

---

<div align="center">
  <strong>NEXUS</strong><br/>
  <sub>Map assets. Track access. Understand the path.</sub>
</div>
