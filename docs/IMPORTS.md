# Import adapter notes

All adapters are passive file importers. Select a format, choose/drop a file or paste text, preview, review conflicts, then import. The parser reads at most 32 MiB per import. A malformed record rejects the import; unsupported records are reported instead of inventing results.

| Adapter | Supported input | Observations |
| --- | --- | --- |
| Nmap | XML `nmaprun`, IPv4 hosts and ports | Addresses, hostnames, OS match, state, service name/product/version |
| httpx | JSON object, array, or JSONL | URL/host/IP, port, HTTP status, title, web server/technology, TLS |
| Nuclei | JSONL | Matched URL/target, template ID, severity, matcher metadata, draft finding and raw evidence |
| Nessus | Nessus v2 XML (`.nessus`) | ReportHost identity, per-port services, plugin IDs, severity, descriptions, solutions and output evidence; port-zero findings attach to the host |
| Burp | Issues XML | Host URL/IP, service, issue type, severity, scanner confidence and retained XML evidence |
| NetExec | Common text rows `PROTOCOL IP PORT HOST [marker] ...` | Host/service observations, explicit valid/invalid authentication, domain/user/secret, administrative marker |

Nmap accepts its exact inert `<!DOCTYPE nmaprun>` declaration; other DTD declarations and entities are refused. Nessus/Burp reject entity declarations and never retrieve external documents. Burp static DTD declarations are accepted without evaluation. Parser text is never evaluated as HTML or commands. A hostname-only HTTP observation is merged only when its identity is unambiguous. Host IP and aliases take precedence; conflicting IPs are not casually merged based on a similar hostname. Importing the same Nmap/httpx fixture twice does not duplicate its host/service identity.

Repeated source observations are still retained as separate import records and provenance. NetExec authentication tests are historical observations; the newest result controls current matrix state. Different secrets for an otherwise identical account become distinct credential records. NetExec needs an unlocked vault because both credentials and the retained raw source are encrypted. Only common recognizable NetExec rows are supported; terminal color/control sequences and tool-version-specific prose may be unsupported. Review parser warnings and use manual entry when needed.

Nuclei, Nessus and Burp findings enter as **Draft** with source evidence. Detection output requires analyst validation. A recognized HTTP/TLS/version observation can complete its corresponding checklist item; importing a port never completes credential testing, exploitation, content discovery, or unrelated methodology checks.

Scanner adapters map recognizable report fields; they do not guarantee every vendor/version extension. Duplicate findings match host, scanner-specific identifier and reproduction text. Nessus findings retain their reported service association, including host-only port-zero observations. Imported HTML fragments remain inert text.

Format references: [Tenable Nessus v2](https://developer.tenable.com/docs/export-file-formats), [PortSwigger XML reports](https://portswigger.net/burp/documentation/desktop/running-scans/reporting/report-settings).

## Fixtures

- `scanner_reports/`: fictional Nessus and Burp reports used by the added integration tests.
- `small_lab/`: two-host Nmap baseline plus HTTP enrichment and a Nuclei finding.
- `internal_ad_lab/`: fictional NetExec-style account observations; no real passwords.
- `pivot_lab/`: reference topology for session/pivot testing.
- `recon_delta_lab/`: changed recon state for comparing observations/snapshots.
- `large_generated_lab/`: deterministic generator instructions; `npm run fixture:large` emits 10,000 hosts and 50,000 services without shipping a large redundant XML file.

The Rust tests exercise each parser, malformed input rejection, merge behavior, provenance, vault protection, and the larger seeded database. Preview remains the required analyst decision point in the UI.
