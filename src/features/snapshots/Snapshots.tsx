import { useState, useEffect } from "react";
import {
  Camera,
  ArrowRight,
  Network,
  Plus,
  Minus,
  RefreshCw,
} from "lucide-react";
import { useWorkspace } from "../../state/workspace";
import { api, type Row, time } from "../../lib/api";
import { PageTitle, Badge, Empty, Modal } from "../../components/ui";
export function Snapshots() {
  const w = useWorkspace();
  const snapshots = w.summary.snapshots;
  const [before, setBefore] = useState(snapshots[1]?.id || "");
  const [after, setAfter] = useState(snapshots[0]?.id || "");
  const [diff, setDiff] = useState<Row[]>([]);
  const [error, setError] = useState("");
  const [create, setCreate] = useState(false);
  const [name, setName] = useState("");
  const [filter, setFilter] = useState("All");
  useEffect(() => {
    if (!before && snapshots[1]) setBefore(snapshots[1].id);
    if (!after && snapshots[0]) setAfter(snapshots[0].id);
  }, [snapshots, before, after]);
  useEffect(() => {
    setError("");
    if (before && after)
      api("diff", { before, after })
        .then(setDiff)
        .catch((e) => setError(e.message));
  }, [before, after, w.revision]);
  return (
    <div className="page">
      <PageTitle
        eyebrow="RECONDELTA"
        title="Snapshots & changes"
        description="Compare meaningful engagement state without timestamp noise."
        actions={
          <button
            className="primary"
            onClick={() => {
              setName(`Assessment state · ${new Date().toLocaleString()}`);
              setCreate(true);
            }}
          >
            <Camera size={15} />
            Create snapshot
          </button>
        }
      />
      {snapshots.length ? (
        <>
          <div className="snapshot-selector">
            <label>
              Baseline
              <select
                aria-label="Baseline snapshot"
                value={before}
                onChange={(e) => setBefore(e.target.value)}
              >
                <option value="">Select baseline</option>
                {snapshots.map((s: Row) => (
                  <option key={s.id} value={s.id}>
                    {s.name} · {time(s.created_at)}
                  </option>
                ))}
              </select>
            </label>
            <ArrowRight size={22} />
            <label>
              Compare with
              <select
                aria-label="Comparison snapshot"
                value={after}
                onChange={(e) => setAfter(e.target.value)}
              >
                <option value="">Select comparison</option>
                {snapshots.map((s: Row) => (
                  <option key={s.id} value={s.id}>
                    {s.name} · {time(s.created_at)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {error && (
            <div role="alert" className="error-box">
              {error}
            </div>
          )}
          {before && after && (
            <>
              <div className="diff-summary">
                {["New", "Changed", "Removed"].map((type) => (
                  <button
                    key={type}
                    onClick={() => setFilter(filter === type ? "All" : type)}
                    className={filter === type ? "active" : ""}
                  >
                    <span
                      className={
                        type === "New"
                          ? "good"
                          : type === "Removed"
                            ? "bad"
                            : "warn"
                      }
                    >
                      {type === "New" ? (
                        <Plus size={18} />
                      ) : type === "Removed" ? (
                        <Minus size={18} />
                      ) : (
                        <RefreshCw size={17} />
                      )}
                    </span>
                    <strong>
                      {diff.filter((d) => d.status === type).length}
                    </strong>
                    <label>{type}</label>
                  </button>
                ))}
                <button
                  className="graph-diff-button"
                  onClick={() => {
                    w.setChanged(diff);
                    w.navigate("Graph");
                  }}
                >
                  <Network size={15} />
                  Show in graph
                </button>
              </div>
              <div className="diff-results">
                {diff.length ? (
                  diff
                    .filter((d) => filter === "All" || d.status === filter)
                    .map((d) => (
                      <details className="diff-item" key={d.entity_id}>
                        <summary>
                          <Badge>{d.status}</Badge>
                          <strong>{d.name || d.entity_id}</strong>
                          <span>{d.kind}</span>
                          <small>{d.fields.length} fields</small>
                        </summary>
                        <div className="diff-fields">
                          <div className="diff-field-head">
                            <span>Field</span>
                            <span>Before</span>
                            <span>After</span>
                          </div>
                          {d.fields.map((f: Row) => (
                            <div className="diff-field" key={f.field}>
                              <strong>{f.field.replaceAll("_", " ")}</strong>
                              <pre>
                                {f.before === null
                                  ? "—"
                                  : typeof f.before === "object"
                                    ? JSON.stringify(f.before)
                                    : String(f.before)}
                              </pre>
                              <pre>
                                {f.after === null
                                  ? "—"
                                  : typeof f.after === "object"
                                    ? JSON.stringify(f.after)
                                    : String(f.after)}
                              </pre>
                            </div>
                          ))}
                        </div>
                        <button
                          className="text-button"
                          onClick={() => w.select(d.entity_id)}
                        >
                          Inspect current record
                        </button>
                      </details>
                    ))
                ) : (
                  <Empty
                    title="No meaningful changes"
                    detail="The selected snapshots have the same observed state."
                  />
                )}
              </div>
            </>
          )}
        </>
      ) : (
        <Empty
          title="Capture a baseline"
          detail="Create a snapshot before a new import or a significant assessment phase."
          action={
            <button onClick={() => setCreate(true)}>Create snapshot</button>
          }
        />
      )}
      <section className="snapshot-history">
        <div className="section-label">
          <h2>Snapshot history</h2>
        </div>
        {snapshots.map((s: Row) => (
          <div className="snapshot-history-row" key={s.id}>
            <Camera size={15} />
            <strong>{s.name}</strong>
            <time>{time(s.created_at)}</time>
            <button className="text-button" onClick={() => setBefore(s.id)}>
              Use as baseline
            </button>
          </div>
        ))}
      </section>
      {create && (
        <Modal
          title="Create snapshot"
          subtitle="Capture assets, services, access, pivots, relationships, findings, and reachability."
          onClose={() => setCreate(false)}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              w.run("snapshot", { name }, "Snapshot created")
                .then((r) => {
                  setBefore(after);
                  setAfter(r.id);
                  setCreate(false);
                })
                .catch(() => {});
            }}
          >
            <label>
              Snapshot name
              <input
                value={name}
                required
                autoFocus
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <footer>
              <button type="button" onClick={() => setCreate(false)}>
                Cancel
              </button>
              <button className="primary">Create snapshot</button>
            </footer>
          </form>
        </Modal>
      )}
    </div>
  );
}
