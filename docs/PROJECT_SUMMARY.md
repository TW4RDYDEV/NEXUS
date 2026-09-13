# NEXUS 1.0 — project summary

NEXUS 1.0.0 is the first public-release source baseline. It is a local-first offensive-security engagement workspace built around a Rust/SQLite core and a React/Tauri desktop interface. Branding: NEXUS · by TWARDY.exe / TW4RDYDEV. Engagement schema: 3.

## Product scope

NEXUS correlates discovery, recorded access, credentials, sessions, pivots, findings, evidence and assessment state without pretending that visibility equals compromise. Imported observations stay attributable to their source, successful access remains explicit, and deterministic coverage/opportunity logic explains why work is suggested.

The application includes isolated engagement workspaces, Scope Guard, normalized asset/service inventories, graph/inspector workflows, encrypted credentials and access history, session/pivot reachability, directed paths, reviewed scanner imports, assessment plans, findings/evidence, reports, timeline, snapshots/ReconDelta, search, backup/recovery and a fictional demo.

## Professional boundaries

The built-in active runner is intentionally narrow: one resolved IPv4 target, an explicit in-scope rule, a fixed Nmap TCP-connect/version profile, bounded ports/output/time and manual import review. NEXUS does not contain autonomous exploitation, opaque AI recommendations, telemetry, cloud synchronization or hidden outbound tracking.

Credential secrets are protected using Argon2id-derived keys and XChaCha20-Poly1305 authenticated encryption. Imported files are treated as untrusted data; scanner text is not executed. Workspace bundles and evidence use integrity metadata, and database changes use SQLite transactions/migrations.

## Release discipline

The source baseline is version-normalized to 1.0.0 across npm, Cargo and Tauri metadata. Stale pre-release binaries are intentionally excluded from the source tree. A Windows release must be created with `npm run release:build` after the project license and release metadata are verified. The release script:

- gates version/branding/authorship consistency;
- refuses the old MIT project license;
- enables Rust `--remap-path-prefix` mappings for the repository, Cargo/Rustup homes and user home;
- stages the portable Windows executable separately from source;
- verifies expected NEXUS markers remain in the compiled executable;
- rejects the staged binary if the current local build paths are embedded;
- emits SHA-256 sums for staged files.

Private authorship/release fingerprint reports stay gitignored. GitHub release binaries should be attached to a release rather than committed into the source tree.

## Verification

The underlying feature-complete baseline previously passed the project's Rust/frontend/browser/security/scale suites. Because release metadata and packaging controls have now been normalized for the first public `1.0.0`, the final native Windows build must rerun the complete verification procedure before publication. See [VERIFICATION](VERIFICATION.md), [RELEASE](RELEASE.md), [AUDIT](AUDIT.md), and [SECURITY](../SECURITY.md).
