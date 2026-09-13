import { useState } from "react";
import { api, chooseDirectory, download, type Row } from "../../lib/api";
import { useWorkspace } from "../../state/workspace";
export function Delivery() {
  const w = useWorkspace();
  const [path, setPath] = useState("");
  const [result, setResult] = useState<Row | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [drafts, setDrafts] = useState(false);
  async function run(op: string, args: Row = {}) {
    setBusy(true);
    setError("");
    setResult(null);
    try {
      const r = await api(op, args);
      setResult(r);
      if (op === "client_report") download("NEXUS-client-report.html", r.text);
      if (op === "recover_bundle") await w.refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Operation failed");
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="settings-section">
      <h2>Reports & complete workspace transfer</h2>
      <div className="setting-row">
        <div>
          <strong>Client report</strong>
          <p>
            Printable HTML with scope, finding details, evidence hashes and
            assessment outcomes. Credential records and attachments are
            excluded. Review analyst text before sharing.
          </p>
          <label className="inline-check">
            <input
              type="checkbox"
              checked={drafts}
              onChange={(e) => setDrafts(e.target.checked)}
            />{" "}
            Include draft findings
          </label>
        </div>
        <button
          disabled={busy}
          onClick={() => run("client_report", { include_drafts: drafts })}
        >
          Export client report
        </button>
      </div>
      <div className="setting-row">
        <div>
          <strong>Complete workspace bundle</strong>
          <p>
            Copies a consistent database, imports and evidence with a SHA-256
            manifest, then verifies every attachment. Vault secrets remain
            encrypted; notes, reports and scanner output retain their existing
            form. Previous exports and rotating backups are excluded.
          </p>
        </div>
        <button disabled={busy} onClick={() => run("workspace_export")}>
          Export complete workspace
        </button>
      </div>
      <div className="setting-row">
        <div>
          <strong>Verify or recover a bundle</strong>
          <p>
            Choose an exported NEXUS-workspace folder. Recovery verifies it and
            opens a separate engagement copy.
          </p>
          <input
            aria-label="Workspace bundle path"
            placeholder="Full path to NEXUS-workspace folder"
            value={path}
            onChange={(e) => setPath(e.target.value)}
          />
        </div>
        <button
          disabled={busy}
          onClick={async () => {
            const p = await chooseDirectory();
            if (p) setPath(p);
          }}
        >
          Choose folder
        </button>
      </div>
      <div className="actions">
        <button
          disabled={busy || !path}
          onClick={() => run("workspace_verify", { path })}
        >
          Verify bundle
        </button>
        <button
          disabled={busy || !path}
          onClick={() => run("recover_bundle", { path })}
        >
          Recover verified bundle
        </button>
      </div>
      {busy && <p role="status">Processing local files…</p>}
      {error && (
        <div role="alert" className="error-box">
          {error}
        </div>
      )}
      {result && (
        <div role="status" className="delivery-result">
          <strong>
            {result.verified ? "Integrity verified" : "Export complete"}
          </strong>
          {result.files && (
            <p>
              {result.files} files · {(result.bytes / 1048576).toFixed(2)} MiB
            </p>
          )}
          {result.path && <p className="mono path-text">{result.path}</p>}
        </div>
      )}
    </section>
  );
}
