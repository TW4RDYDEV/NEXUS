# Security model

NEXUS is a local analyst workspace. Only run assessment tools against systems you are authorized to test. The application does not infer authorization from discovered assets or imported results.

## Secrets

Credential secrets and retained NetExec source are encrypted with XChaCha20-Poly1305 using a random 24-byte nonce per encryption. Entity identifiers are authenticated as associated data. Argon2id derives a 256-bit key from a passphrase and random 16-byte workspace salt. The implementation uses maintained cryptographic crates rather than custom algorithms. Argon2's v0.5 defaults are used: version 0x13, 19,456 KiB memory, two passes, and one lane.

The passphrase and derived key are not stored on disk. A derived key exists while the vault is unlocked, is zeroized on lock/drop, and expires after configurable vault inactivity. Ciphertext is excluded from renderer inventory/search/snapshot results. Secrets remain masked until explicitly revealed, and the renderer clears that reveal after 15 seconds or window blur. Wrong-passphrase attempts leave the vault locked. Exact secret whitespace is preserved. NetExec observations with different secrets for the same identity create separate credential records.

This protects secrets at rest against possession of the workspace without its passphrase. It does not protect against malware, an administrator, debugger, memory scraping, weak passphrases, screenshots of intentional reveals, or clipboard readers. A copied secret remains in the operating-system clipboard until replaced. JavaScript and IPC can contain a secret transiently during entry/reveal; JavaScript strings cannot be reliably zeroized. There is no passphrase recovery or reset that decrypts existing secrets.

The rest of SQLite is not encrypted. Hostnames, account identifiers, findings, notes, and evidence are sensitive engagement metadata. Non-NetExec imports and evidence retain their actual content and may themselves contain secrets; use disk encryption and appropriate filesystem access controls. Do not put production credentials into notes or fixture files. Backup copies retain the same encrypted vault state.

## IPC and storage

- The renderer has no general shell or filesystem capability. It uses a Rust command dispatcher with an explicit operation/table/column boundary and SQL parameters for values.
- Reference validation prevents service/host mismatches and invalid pivot/session references. Manual edits cannot set managed evidence paths, hashes, timestamps, or credential ciphertext.
- Attachments are limited to 20 MiB; import content is limited to 32 MiB; IPC requests are limited to 50 MiB. Internal attachment names are generated IDs.
- Evidence reads/exports resolve paths within the engagement evidence directory and verify SHA-256. Files are never automatically executed. Text is rendered as text, not HTML. Only verified PNG/JPEG files are shown inline.
- The CSP forbids external script, object, and frame sources. Only the main window has core/dialog capabilities. The packaged application has no localhost server.
- The browser bridge is development-only, bound to loopback, and checks same-origin requests plus a custom request header. It is intended for a trusted development machine, not a multi-user service.
- XML entity declarations and malformed JSON/JSONL are rejected. Live Nmap's exact inert `<!DOCTYPE nmaprun>` is accepted; Burp's static DTD can be parsed without external fetching or entity declarations. A failed import rolls back database changes; conflicting nonempty observations require an explicit acceptance to replace existing values.

## Active execution

Only Nmap is executable through the built-in runner. Other integrations import files. The runner builds a direct child process with fixed arguments, never shell interpolation. It accepts a single host, resolves to one IPv4 address, checks includes and exclusions against that address and hostname, and pins the command to the validated address. CIDR/wildcard scan requests and arbitrary arguments are not accepted. The profile uses a TCP connect scan with explicit unprivileged mode and no reverse DNS, light version detection, a bounded port list, and host/process/output limits. Cancellation kills and waits for the child. Closing the application also drops a running child.

Imports are passive observations and can retain out-of-scope entities; this never grants permission for an active scan. Opening, creating, or recovering another engagement is blocked while a scan is active.

## Backup and integrity

The database uses SQLite WAL and FULL synchronous writes. Pre-import and periodic backups use SQLite's online backup API. Restoring creates a separate workspace, preserving the original. Backups are local copies, not protection against device loss or malicious tampering. Evidence hashes identify byte changes, not the authenticity of the original observation. Snapshot diffs and directed access paths reflect recorded facts, not independent proof of exploitability.

Complete workspace bundles verify a manifest, SQLite integrity and foreign keys, required files and attachment hashes before recovery. Retained NetExec ciphertext is verified against the bundle manifest while its plaintext source hash remains for unlocked interpretation. Bundle hashes are integrity checks, not signatures. Client HTML reports escape imported/analyst text, disable scripts and remote resources, and omit credential records; analyst-authored text can still contain secrets.

## Dependency status and reporting

Read [docs/AUDIT.md](docs/AUDIT.md) for exact audit results and residual upstream advisories. Windows is the intended first public release target; the final native 1.0.0 artifact must be rebuilt and verified before publication. Report suspected vulnerabilities privately to the maintainer through your established contact, with reproduction steps and a fictional minimal workspace; do not send real engagement data or secrets. No public reporting mailbox or hosted support service is configured in this source distribution.
