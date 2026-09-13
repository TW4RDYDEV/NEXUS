import { useEffect, useState } from "react";
import { Plus, ListChecks } from "lucide-react";
import { api, type Row } from "../../lib/api";
import { useWorkspace } from "../../state/workspace";
import { PageTitle, Badge, Modal, Empty } from "../../components/ui";
import { ReferenceSelect } from "../../components/ReferenceSelect";
export function Assessment() {
  const w = useWorkspace();
  const [data, setData] = useState<Row | null>(null);
  const [q, setQ] = useState("");
  const [area, setArea] = useState("");
  const [state, setState] = useState("");
  const [page, setPage] = useState(0);
  const [error, setError] = useState("");
  const [adding, setAdding] = useState(false);
  const [template, setTemplate] = useState("web-api");
  const [asset, setAsset] = useState("");
  const [owner, setOwner] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let live = true;
    const timer = setTimeout(
      () =>
        api("assessment_plan", { q, area, state, offset: page * 50 })
          .then((r) => {
            if (live) {
              setData(r);
              setError("");
            }
          })
          .catch((e) => {
            if (live) setError(e.message);
          }),
      150,
    );
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [q, area, state, page, w.revision]);
  const counts = data?.counts;
  return (
    <div className="page">
      <PageTitle
        eyebrow="ASSESSMENT OPERATIONS"
        title="Assessment plan"
        description="Track scope, ownership and validation across testing domains. Every outcome records an analyst decision."
        actions={
          <>
            <button onClick={() => w.edit("assessment_checks")}>
              <Plus size={14} />
              Custom check
            </button>
            <button className="primary" onClick={() => setAdding(true)}>
              <ListChecks size={14} />
              Add methodology
            </button>
          </>
        }
      />
      <div className="reach-summary">
        <span>
          <strong>{counts?.total || 0}</strong> planned
        </span>
        <span>
          <strong>{counts?.completed || 0}</strong> validated
        </span>
        <span>
          <strong>{counts?.failed || 0}</strong> failed
        </span>
        <span>
          <strong>{counts?.blocked || 0}</strong> blocked
        </span>
        <span>
          <strong>{counts?.overdue || 0}</strong> overdue
        </span>
        <span>{counts?.excluded || 0} not applicable</span>
      </div>
      <div className="table-toolbar">
        <input
          aria-label="Search assessment checks"
          placeholder="Search checks, objectives or owners…"
          maxLength={200}
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setPage(0);
          }}
        />
        <select
          aria-label="Assessment area"
          value={area}
          onChange={(e) => {
            setArea(e.target.value);
            setPage(0);
          }}
        >
          <option value="">All domains</option>
          {data?.areas.map((r: Row) => (
            <option key={r.area}>{r.area}</option>
          ))}
        </select>
        <select
          aria-label="Assessment state"
          value={state}
          onChange={(e) => {
            setState(e.target.value);
            setPage(0);
          }}
        >
          <option value="">All states</option>
          {[
            "Not started",
            "In progress",
            "Blocked",
            "Passed",
            "Failed",
            "Not applicable",
          ].map((v) => (
            <option key={v}>{v}</option>
          ))}
        </select>
      </div>
      {error && (
        <div role="alert" className="error-box">
          {error}
        </div>
      )}
      {data?.rows.length ? (
        <div className="assessment-checks">
          {data.rows.map((r: Row) => (
            <article className="assessment-check" key={r.id}>
              <div>
                <span className="eyebrow">
                  {r.area} · {r.priority}
                </span>
                <h3>
                  <button className="table-name" onClick={() => w.select(r.id)}>
                    {r.name}
                  </button>
                </h3>
                <p>{r.objective}</p>
                <div className="assessment-meta">
                  <span>{r.owner || "Unassigned"}</span>
                  <span>
                    {r.reference_labels?.asset_id || "Engagement-wide"}
                  </span>
                  {r.due_date && <span>Due {r.due_date}</span>}
                  {r.evidence_id && (
                    <button
                      className="text-button"
                      onClick={() => w.select(r.evidence_id)}
                    >
                      Evidence linked
                    </button>
                  )}
                  {r.finding_id && (
                    <button
                      className="text-button"
                      onClick={() => w.select(r.finding_id)}
                    >
                      Finding linked
                    </button>
                  )}
                </div>
                {r.result && <p className="assessment-result">{r.result}</p>}
              </div>
              <div className="assessment-actions">
                <Badge>{r.state}</Badge>
                <button onClick={() => w.edit("assessment_checks", r)}>
                  Update check
                </button>
                <button
                  className="text-button"
                  onClick={() => w.edit("evidence", { entity_id: r.id })}
                >
                  Attach evidence
                </button>
              </div>
            </article>
          ))}
        </div>
      ) : (
        <Empty
          title="Build a deliberate assessment plan"
          detail="Add a domain methodology or create a custom check. Templates are planning prompts, not proof that testing occurred."
          action={
            <button onClick={() => setAdding(true)}>Choose methodology</button>
          }
        />
      )}
      <div className="paged-controls">
        <span>{data?.total || 0} matching checks</span>
        <button disabled={!page} onClick={() => setPage(page - 1)}>
          Previous checks
        </button>
        <button
          disabled={(page + 1) * 50 >= (data?.total || 0)}
          onClick={() => setPage(page + 1)}
        >
          Next checks
        </button>
      </div>
      {adding && (
        <Modal
          title="Add assessment methodology"
          subtitle="Existing template checks on the same asset are preserved. New checks start as Not started."
          onClose={() => setAdding(false)}
        >
          <form
            onSubmit={async (e) => {
              e.preventDefault();
              setBusy(true);
              try {
                const r = await w.run("assessment_template", {
                  template_id: template,
                  asset_id: asset,
                  owner,
                });
                w.notify(r.added + " checks added");
                setAdding(false);
              } catch (e) {
                setError(
                  e instanceof Error ? e.message : "Unable to add methodology",
                );
              } finally {
                setBusy(false);
              }
            }}
          >
            <label>
              Methodology
              <select
                aria-label="Methodology"
                value={template}
                onChange={(e) => setTemplate(e.target.value)}
              >
                {data?.templates.map((t: Row) => (
                  <option value={t.id} key={t.id}>
                    {t.name} · {t.checks.length} checks
                  </option>
                ))}
              </select>
            </label>
            <label>
              Owner
              <input
                value={owner}
                onChange={(e) => setOwner(e.target.value)}
                maxLength={200}
              />
            </label>
            <label>
              Target (optional)
              <ReferenceSelect
                table="assets"
                label="Assessment target"
                value={asset}
                onChange={setAsset}
              />
            </label>
            <p className="secondary small">
              Tailor these starting points to the rules of engagement. Passed
              and Failed require a recorded result; Blocked and Not applicable
              require a reason.
            </p>
            <footer>
              <button type="button" onClick={() => setAdding(false)}>
                Cancel
              </button>
              <button className="primary" disabled={busy}>
                {busy ? "Adding…" : "Add checks"}
              </button>
            </footer>
          </form>
        </Modal>
      )}
    </div>
  );
}
