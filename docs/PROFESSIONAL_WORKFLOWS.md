# Professional assessment workflows

NEXUS is a local assessment workspace. It connects discovery, recorded access, testing decisions and evidence. Use your preferred specialist tools for testing; import results or record observations here. The built-in active runner is a scoped Nmap TCP-connect profile. Templates do not execute attacks, certify coverage or replace a methodology tailored to the engagement.

## Start and organize

1. Start a verified NEXUS native build, or use `npm run desktop` from the source workspace during development. Public release artifacts are staged separately from source and require Windows x64 plus WebView2 Runtime.
2. Try **Explore demo**. It creates an isolated fictional engagement; the vault passphrase is `nexus-demo-only`. Then create a separate engagement for your own work.
3. Set the client/project, description and dates. Record authorization, contacts, test windows, stop conditions and reporting requirements in the engagement context and the Engagement operations methodology. Define includes/exclusions under **Scope Guard**.
4. Import Nmap, httpx, Nuclei, NetExec, Nessus or Burp observations. Review identities, conflicts and warnings before committing. Imported findings start as Draft. Discovery never grants permission to scan.

## Plan and validate

Open **Assessment → Add methodology**. There are 52 original starting checks across ten domains: Web & API; Directory & identity; Cloud & SaaS; Network & infrastructure; Containers & Kubernetes; Mobile applications; Wireless & physical interfaces; Desktop & thick clients; Engagement operations; OT & embedded systems.

Apply a template to the whole engagement or one asset and assign an owner. Applying the same template to the same target again preserves existing checks and skips duplicates. You can add custom checks, edit objectives, assign priorities and due dates, or mark a check In progress. These owner fields are coordination notes, not authenticated team accounts.

Use **Update check** to record Passed or Failed with the observed result. Blocked and Not applicable require a reason. Link evidence and an associated finding. An outcome is an analyst assertion supported by the recorded evidence, not a tool-generated proof. Failed checks and confirmed findings are separate decisions. The plan lists unfinished work and overdue checks; snapshots include meaningful plan changes.

For web testing the reference links to the [OWASP Web Security Testing Guide](https://owasp.org/www-project-web-security-testing-guide/stable/4-Web_Application_Security_Testing/). The other domain plans are NEXUS starting prompts, not reproductions or certifications of external standards. Expand them for the actual systems, risks and agreed methods.

## Work with larger inventories

- Reference fields search the full database and page through 50 matches. Search by name, address, protocol, context or other displayed detail. An existing selection remains available when it is outside the search page.
- The credential list searches and pages all identities. The matrix pages 25 hosts and retrieves the latest result for each displayed identity/service pair. Test history pages 50 records. The first 200 open services per host are rendered; **search all** opens the recording form with a full-database service picker. This display limit is explicit.
- Inventory service counts and finding evidence counts come from SQLite. Inspector reference labels and path labels resolve independently of the small display cache.
- The graph browses 60 matching assets at a time, bounds its database neighborhood, and draws at most 500 nodes/4,000 edges. Search across the engagement, move between pages or focus an entity. A focused service remains visible even when the general service layer is hidden. The Attack Path view includes the recorded path's entities. A bounded picture is not a full-network enumeration.
- Pivot pages show 20 routes, each with a count of all known assets on the route and the first 20 host chips. Inventory search remains available for the complete host set.
- Suggestions are a bounded shortlist, up to 50 per category; the overview shows seven. Use full inventories and assessment/coverage pages to reconcile all outstanding work.

The included scale test covers 10,000 hosts, 50,000 services and 100,000 relationships. It measures actual database queries, not a guarantee for every hardware/workload. Full snapshot capture, imports, reports and bundles still scale with the data being processed. Very large imports can occupy the local backend while they are reviewed/committed. There is no claim of unlimited scale.

## Report and transfer

**Settings → Security → Export client report** creates printable HTML with context, scope/exclusions, assessment status counts, findings, evidence identifiers/hashes, remediation and unfinished work. Draft findings are excluded unless selected. The report excludes credential inventory and attachment bytes. Analyst text and imported scanner descriptions can contain sensitive material; review the report before sharing. HTML is escaped and its CSP blocks scripts and external resources. Open the exported HTML in a browser and print/save as PDF if needed.

**Export complete workspace** creates a uniquely named folder under the engagement's exports directory containing a consistent SQLite backup, workspace metadata, imports, evidence and a SHA-256 manifest. It excludes previous exports and rotating database backups to avoid recursive growth. This is a complete active workspace transfer, not a backup of every historical exported file. Copy the entire folder to the intended backup location.

Choose that folder in **Verify bundle** to check file sizes/hashes, database integrity/references and attachment coverage. **Recover verified bundle** verifies and copies it into a separate engagement. Encrypted vault records and retained NetExec sources stay encrypted and require the original passphrase. A NetExec import's stored source hash identifies plaintext; its transferred ciphertext is verified against the bundle manifest without unlocking. SQLite metadata, notes and other attachments retain their existing encryption state.

Hashes detect changes relative to a manifest; they are not a digital signature or proof of authorship. A person who can replace both files and manifest can recompute hashes. Keep authoritative copies and use protected storage appropriate to your engagement. Recovery never modifies the source engagement.

## Release operation and limits

The portable executable remains unsigned because no trusted publisher certificate was supplied. `scripts/sign-windows.ps1` signs a staged build using a real certificate from the maintainer's Windows certificate store and verifies it; it does not generate a pretend publisher identity. Regenerate the manifest and ZIP hash after signing. See [RELEASE](RELEASE.md).

This release has automated tests and a live localhost Nmap check, not independent security certification or field validation across every attack type. It has no concurrent multi-user collaboration, remote agents, cloud synchronization or autonomous exploitation. Its broad usefulness is in organizing, tracing and reviewing professional assessments alongside specialist tools. See [VERIFICATION](VERIFICATION.md) and [AUDIT](AUDIT.md) for exact evidence and remaining upstream warnings.
