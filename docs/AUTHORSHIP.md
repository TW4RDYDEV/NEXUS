# NEXUS authorship markers

NEXUS — TWARDY.exe / TW4RDYDEV

These public architectural markers help maintainers compare code origin, recognize redistributed derivatives and retain local reference snapshots. They are static, readable and removable. They are not proof of infringement, a secret watermark, DRM or a restriction on legitimate private modifications. Matching markers and hashes are evidence to interpret alongside provenance and release history, not an automatic ownership verdict.

## Public marker layers

| Layer | Source of truth / purpose |
| --- | --- |
| Product identity | `src-tauri/src/identity.rs`: `nexus.tw4rdy.core`, `nexus-engagement`, `nx-engagement-graph` |
| SQLite format | Application ID `0x4E585553` / `1314411859`, `nx_format_metadata`, `nx_migrations`; see `architecture/database-format.md` |
| Workspace and serialized state | `.nexus/workspace.json`, snapshot envelopes, graph and summary format metadata |
| Typed events | `DomainEvent` in `src-tauri/src/domain/mod.rs`; persisted `nx.*` timeline events and separate human labels |
| Typed graph relationships | `RelationshipKind` in the same module; ten `NX_*` canonical values and clean labels |
| Support errors | `ErrorCode` and serializable `NexusError` in `src-tauri/src/errors.rs`; actual errors displayed as `[NX-…] message` |
| Provenance | `DiscoveryRecord`, `ObservationSource`, `ObservationConfidence`, `NormalizationRecord`; used by parser, entity resolution, importer and analyst edits |
| Engine decomposition | Resolution, reachability, confirmed directed paths, coverage and meaningful-state ReconDelta remain ordinary maintainable domain functions |
| Build metadata | About and local diagnostic export: product, product ID, application/schema versions, format family, optional build-time Git commit, build mode |
| Binary strings | Product identifiers naturally retained because compiled code serves real format/build metadata |
| Canonical fictional environment | `fixtures/canonical-nexus-lab.json`: `nx-demo-helios-meridian-v1`, Helios / Meridian, `helios.lab`, `NX-DEMO-WEB02`, `NX-DEMO-FILE01`, `NX-DEMO-DC01`, `svc_meridian`, `svc_archive` |
| Developer reference hashes | Explicitly invoked scripts below; source sections and sorted first-party file fingerprints |
| Source attribution | Concise first-party module/document authorship lines; no legal license decision in this change |

### Stable vocabularies

Relationships: `NX_EXPOSES`, `NX_RESOLVES_TO`, `NX_AUTHENTICATES_TO`, `NX_DISCOVERED_FROM`, `NX_HAS_SESSION`, `NX_REACHABLE_VIA`, `NX_AFFECTED_BY`, `NX_EVIDENCED_BY`, `NX_CONNECTS_TO`, `NX_MEMBER_OF`.

Core events: `nx.asset.discovered`, `nx.asset.enriched`, `nx.service.observed`, `nx.credential.recorded`, `nx.authentication.confirmed`, `nx.session.opened`, `nx.pivot.created`, `nx.network.reachable`, `nx.finding.confirmed`, `nx.snapshot.created`, `nx.import.completed`. Additional real audit events cover edits, vault state, imports, backup/recovery and local runner activity. A draft finding or invalid authentication never emits a confirmed-success event. Reachability transitions are recorded after successful writes and inside the same transaction.

| Code | Real condition |
| --- | --- |
| NX-SCP-2749 | Scope rule or scan validation failure |
| NX-IMP-2417 | Import parsing or commit failure |
| NX-IMP-5931 | Oversized import |
| NX-GPH-3802 | Graph, path or relationship failure |
| NX-VLT-4826 | Vault or credential failure |
| NX-PVT-7314 | Pivot host differs from its source session |
| NX-RCH-8642 | Invalid IPv4 pivot network |
| NX-DB-4283 | Storage or general request failure |
| NX-DB-9076 | Unsupported database/workspace format |
| NX-DB-3568 | Database integrity failure |
| NX-SNP-6193 | Snapshot creation or comparison failure |
| NX-COV-8251 | Coverage read or validation failure |

These codes are fixed enum variants, not runtime random numbers. IPC returns `{code, message}`. Older internal string-result helpers keep their existing contract; explicit registered prefixes retain their code at the boundary, and remaining failures receive a stable operation-category code. Human error prose is not pattern-matched to infer conditions.

### Canonical demo

The private-address topology starts at Operator → NX-DEMO-WEB02 (`10.20.4.17`) → `svc_archive` → NX-DEMO-FILE01 (`172.22.20.12`). PIVOT-01 makes the first internal segment reachable; PIVOT-02 reaches `172.22.40.0/24`, including NX-DEMO-DC01 (`172.22.40.10`). A separate recorded `lab_admin` credential confirms DC access. Reachability alone never claims successful authentication. NX-DEMO-ARCHIVE01 (`172.22.60.9`) remains unreachable. Snapshot 1 precedes the second hop; snapshot 2 captures validated internal access. Parser XML/JSONL/NetExec fixtures, backend workflow tests and browser tests share this environment. All identities and observations are fictional. Existing user engagements are never renamed.

## Local maintainer commands

Run from the NEXUS source directory with Node.js:

```sh
npm run authorship:check
npm run authorship:report
npm run fingerprint:release
npm run fingerprint:test
```

- `scripts/authorship_manifest.mjs` verifies expected source markers. `--report` writes `authorship-fingerprint-report.json` with extracted Rust identifiers, schema metadata, complete error/event/relationship/migration registries, fixture identifiers and six selected stable source-section SHA-256 hashes.
- `scripts/create_release_fingerprint.mjs` writes `release-fingerprints/NEXUS-1.0.0.json` (version from `package.json`). It hashes an explicit first-party selection and the sorted list itself. UTF-8 text is normalized from CRLF/CR to LF, paths use forward slashes, and no timestamp is added. Dependencies, lockfiles, builds, binaries, generated large fixtures, screenshots, audit outputs and private reports are excluded. Normalizing whitespace other than line endings is deliberately avoided.
- `scripts/fingerprint-lib.mjs` holds shared selection/extraction rules. The six documented source section selectors cover identity constants, domain events, reachability, coverage, ReconDelta and entity resolution. A moved selector causes an explicit failure so a maintainer can review its new boundary.
- Both reports use only a locally available Git HEAD hash, and only if this source directory owns that repository. No parent workspace commit, remote URL, Git author identity, machine name, environment dump or absolute path is included. Source archives without `.git` report unavailable/null. Build metadata follows the same project ownership rule.

Generated evidence is **private local maintainer output**. Both paths are in `.gitignore` and must be excluded from all release archives. The distributable includes the scripts and this public documentation, never the generated reports. Nothing publishes or sends these reports. Re-running the release script with unchanged selected content and Git commit produces identical bytes. A SHA-256 match indicates equal selected normalized content; it does not establish authorship by itself.

## Verification and privacy

`src-tauri/tests/authorship.rs` tests application ID including SQLite header bytes; canonical identifiers; workspace metadata; exact migration names; legacy data/undo/ReconDelta compatibility; idempotent reopen; foreign and future format handling; private attribution preservation; diagnostics allowlist; real error codes and event behavior; canonical demo graph/access/pivot/snapshot flows. Frontend tests verify structured errors, readable graph vocabulary, About and local diagnostic download. Script tests verify deterministic hashing and output exclusions.

No telemetry, web beacons, tracking pixels, callback URLs, DNS callbacks, network clients, installation identifiers, device fingerprinting, kill switches, obfuscation, anti-debugging, sabotage or anti-removal mechanism is introduced. The scripts use local filesystem operations and read-only local Git commands. Existing explicitly requested assessment scans retain their original behavior. This authorship system is separate from the NEXUS Source-Available License; third-party dependency license identifiers remain untouched.
