# Dependency audit

Refreshed 2026-09-10. The exact lockfiles and machine-readable audit reports are included. No advisories were suppressed.

## Results

- npm audit: **0 vulnerabilities** in the resolved JavaScript dependency tree.
- cargo-audit 0.22.2: **0 vulnerability findings**, **7 informational warnings**, 480 locked packages, 1243 advisory records.
- Rust audit used the freshly downloaded RustSec advisory database with --no-fetch and --no-yanked. The earlier 2026-09-09 separate check compared all 479 registry packages against sparse-index metadata fetched during this build: zero yanked versions and zero missing records. This is a cache-based check; the raw result is in `audits/registry-yanks.json`.

| Package | Classification | Advisory |
| --- | --- | --- |
| proc-macro-error 1.0.4 | unmaintained | [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) |
| unic-char-property 0.9.0 | unmaintained | [RUSTSEC-2025-0081](https://rustsec.org/advisories/RUSTSEC-2025-0081.html) |
| unic-char-range 0.9.0 | unmaintained | [RUSTSEC-2025-0075](https://rustsec.org/advisories/RUSTSEC-2025-0075.html) |
| unic-common 0.9.0 | unmaintained | [RUSTSEC-2025-0080](https://rustsec.org/advisories/RUSTSEC-2025-0080.html) |
| unic-ucd-ident 0.9.0 | unmaintained | [RUSTSEC-2025-0100](https://rustsec.org/advisories/RUSTSEC-2025-0100.html) |
| unic-ucd-version 0.9.0 | unmaintained | [RUSTSEC-2025-0098](https://rustsec.org/advisories/RUSTSEC-2025-0098.html) |
| glib 0.18.5 | unsound | [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) |

The glib warning concerns VariantStrIter unsoundness in the Linux GTK dependency path; it is not part of the Windows target dependency graph. This was verified with cargo tree --features desktop --target x86_64-pc-windows-gnu -i glib (no matching target dependency). It remains relevant before a future Linux release.

The unmaintained packages are transitive dependencies of the Tauri ecosystem. The exact Windows chain was rechecked: Tauri 2.11.5 → tauri-utils 2.9.3 → urlpattern 0.3.0 → UNIC 0.9.0. No advisories were suppressed and no unreviewed local dependency fork was substituted to hide them. The Windows build does not use glib or proc-macro-error 1.x at runtime; the UNIC dependencies still require upstream replacement. This report is not a claim that every dependency is free of defects. Upgrade the upstream Tauri/GTK chain and re-audit before producing additional platform releases.

## Application review

The source baseline includes tests for secret encryption/AAD, exact secret preservation, locked inventory boundaries, encrypted NetExec retention, changed-secret identity, malformed import rollback, managed-field rejection, evidence tamper detection, directed path validation, scope blocking, restart isolation, and recovery copies. No real engagement data is shipped.

Repository scans checked TODO/FIXME/lorem, debug console output, secret-key patterns, and inappropriate placeholder/mock remnants. HTML input placeholder attributes and test fixture terminology are intentional. Runtime credentials only appear in explicitly fictional fixtures or tests.

The scoped Nmap runner's blocked-target path is tested. During pre-release baseline validation, a real Nmap 7.991 scan of a temporary test-owned 127.0.0.1 service passed the runner → preview → import workflow on 2026-09-10. The downloaded official Windows installer had a valid Nmap Software LLC Authenticode signature; it was extracted locally without installing Npcap. Nmap/httpx/Nuclei/NetExec executables are not bundled.

See [SECURITY](../SECURITY.md) for encryption boundaries, clipboard behavior, unencrypted metadata, and the local threat model.
