import { useRef, useState } from "react";
import { Upload, AlertTriangle } from "lucide-react";
import { Modal, Badge } from "../../components/ui";
import { api, chooseFile, type Row } from "../../lib/api";
import { isTauri } from "@tauri-apps/api/core";
import { useWorkspace } from "../../state/workspace";
export function ImportDialog({
  onClose,
  initial,
}: {
  onClose: () => void;
  initial?: { name: string; text: string };
}) {
  const w = useWorkspace();
  const [tool, setTool] = useState("Nmap");
  const [name, setName] = useState(initial?.name || "");
  const [text, setText] = useState(initial?.text || "");
  const [preview, setPreview] = useState<Row | null>(null);
  const [accept, setAccept] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [review, setReview] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  async function read(file: File) {
    if (file.size > 32 * 1024 * 1024) {
      setError("Import exceeds 32 MiB");
      return;
    }
    setName(file.name);
    setText(await file.text());
    setPreview(null);
    setError("");
    if (file.name.endsWith(".nessus")) setTool("Nessus");
    else if (file.name.toLowerCase().includes("burp")) setTool("Burp");
    else if (file.name.endsWith(".xml")) setTool("Nmap");
    else if (file.name.toLowerCase().includes("httpx")) setTool("httpx");
    else if (file.name.toLowerCase().includes("nuclei")) setTool("Nuclei");
    else if (file.name.toLowerCase().includes("netexec")) setTool("NetExec");
  }
  async function choose() {
    if (isTauri()) {
      const path = await chooseFile();
      if (path) {
        const result = await api("read_import", { path });
        setName(result.name);
        setText(result.text);
        setPreview(null);
      }
    } else input.current?.click();
  }
  async function inspect() {
    setError("");
    setBusy(true);
    try {
      const result = await api("preview", { tool, text });
      setPreview(result);
      setAccept([]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to parse file");
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal
      title="Import observations"
      subtitle="Review normalized entities and conflicts before committing a transaction."
      onClose={onClose}
      wide
    >
      <div className="import-options">
        <label>
          Adapter
          <select
            value={tool}
            onChange={(e) => {
              setTool(e.target.value);
              setPreview(null);
            }}
          >
            {["Nmap", "httpx", "Nuclei", "NetExec", "Nessus", "Burp"].map(
              (t) => (
                <option key={t}>{t}</option>
              ),
            )}
          </select>
        </label>
        <label>
          Source filename
          <input value={name} onChange={(e) => setName(e.target.value)} />
        </label>
      </div>
      <input
        ref={input}
        type="file"
        hidden
        accept=".xml,.nessus,.json,.jsonl,.txt"
        onChange={(e) => {
          if (e.target.files?.[0]) read(e.target.files[0]);
        }}
      />
      <button
        className="import-drop"
        onClick={() => choose().catch((e) => setError(e.message))}
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => {
          e.preventDefault();
          if (e.dataTransfer.files[0]) read(e.dataTransfer.files[0]);
        }}
      >
        <Upload size={25} />
        <strong>{name || "Choose or drop an observation file"}</strong>
        <span>
          Nmap XML · httpx JSON / JSONL · Nuclei JSONL · NetExec text · 32 MiB
          maximum
        </span>
      </button>
      <details className="paste-import">
        <summary>Paste raw observations</summary>
        <textarea
          aria-label="Raw import content"
          spellCheck={false}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setPreview(null);
          }}
          rows={6}
        />
      </details>
      {preview && (
        <div className="import-preview">
          <div className="section-label">
            <h3>Import preview</h3>
            <Badge>Ready for review</Badge>
          </div>
          <div className="import-counts">
            {[
              ["hosts", "Hosts"],
              ["services", "Services"],
              ["existing", "Existing matches"],
              ["new_entities", "New hosts"],
              ["updates", "Service matches"],
            ].map(([key, label]) => (
              <div key={key}>
                <strong>{preview[key]}</strong>
                <span>{label}</span>
              </div>
            ))}
          </div>
          {preview.conflicts.length > 0 && (
            <div className="conflict-review">
              <h3>{preview.conflicts.length} conflicting observations</h3>
              <p>
                Existing values are retained unless you explicitly accept an
                incoming value.
              </p>
              {preview.conflicts.map((c: Row) => (
                <label
                  className="conflict-row"
                  key={c.key + String(c.incoming)}
                >
                  <input
                    type="checkbox"
                    checked={accept.includes(c.key)}
                    onChange={(e) =>
                      setAccept(
                        e.target.checked
                          ? [...accept, c.key]
                          : accept.filter((k) => k !== c.key),
                      )
                    }
                  />
                  <div>
                    <strong>
                      {c.entity} · {c.field}
                    </strong>
                    <span>
                      <del>{c.current}</del> → {c.incoming}
                    </span>
                  </div>
                  <span>Accept incoming</span>
                </label>
              ))}
            </div>
          )}
          {preview.warnings.map((warn: string, i: number) => (
            <p className="warning-line" key={i}>
              <AlertTriangle size={14} />
              {warn}
            </p>
          ))}
          {preview.requires_vault && !w.summary.vault_unlocked && (
            <div className="error-box">
              Unlock the vault before importing NetExec. Its original output is
              retained encrypted.{" "}
              <button className="text-button" onClick={w.openVault}>
                Unlock vault
              </button>
            </div>
          )}
          <button className="text-button" onClick={() => setReview(!review)}>
            {review ? "Hide" : "Review"} entity plan
          </button>
          {review && (
            <div className="import-review-rows">
              {preview.review.map((r: Row, i: number) => (
                <div key={i}>
                  <span className="mono">{r.target}</span>
                  <span>{r.action}</span>
                  <span>{r.services} services</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      {error && (
        <div role="alert" className="error-box">
          {error}
        </div>
      )}
      <footer>
        <span className="secondary small">
          Original source retained · transactional writes
        </span>
        <button onClick={onClose}>Cancel</button>
        {preview ? (
          <button
            className="primary"
            disabled={
              busy || (preview.requires_vault && !w.summary.vault_unlocked)
            }
            onClick={async () => {
              setBusy(true);
              try {
                await w.run(
                  "import",
                  {
                    tool,
                    name: name || `${tool.toLowerCase()}-observations`,
                    text,
                    accept,
                  },
                  "Observations imported",
                );
                setText("");
                onClose();
              } catch (e) {
                setError(e instanceof Error ? e.message : "Import failed");
              } finally {
                setBusy(false);
              }
            }}
          >
            {busy ? "Importing…" : "Import observations"}
          </button>
        ) : (
          <button
            className="primary"
            disabled={busy || !text.trim()}
            onClick={inspect}
          >
            {busy ? "Parsing…" : "Preview import"}
          </button>
        )}
      </footer>
    </Modal>
  );
}
