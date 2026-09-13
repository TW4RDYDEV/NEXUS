import {
  ArrowUpRight,
  ArrowRight,
  Network,
  Plus,
  Upload,
  Route,
} from "lucide-react";
import { useWorkspace, entityName } from "../../state/workspace";
import { Badge, PageTitle, Empty } from "../../components/ui";
import { type Row, time } from "../../lib/api";
export function Overview() {
  const w = useWorkspace();
  const s = w.summary;
  const counts = s.counts;
  const path = s.path as string[];
  const assets = w.lookup.assets.filter((a) => a.archived !== 1);
  return (
    <div className="page overview">
      <PageTitle
        eyebrow={`${s.engagement.demo ? "DEMO DATA / " : ""}OPERATIONAL BRIEFING`}
        title={s.engagement.name}
        description={s.engagement.description || "Your engagement, in context."}
        actions={
          <>
            <button onClick={() => w.edit("engagement", s.engagement)}>
              Engagement details
            </button>
            <button className="primary" onClick={w.importFile}>
              <Upload size={14} />
              Import observations
            </button>
          </>
        }
      />
      <div className="engagement-meta">
        <Badge>{s.engagement.status}</Badge>
        <span>{s.engagement.kind}</span>
        <span>Started {time(s.engagement.start_date)}</span>
        {s.engagement.client && <span>{s.engagement.client}</span>}
        <span className="local-note">
          <span className="status-dot" />
          Local workspace
        </span>
      </div>
      <div className="briefing-counts">
        {[
          ["assets", "Assets discovered", "Assets"],
          ["reachable", "Reachable", "Pivots"],
          ["accessed", "Accessed", "Sessions"],
          ["privileged", "Privileged", "Sessions"],
          ["credentials", "Credentials", "Credentials"],
          ["findings", "Findings", "Findings"],
        ].map(([key, name, view]) => (
          <button key={key} onClick={() => w.navigate(view as any)}>
            <span>{Number(counts[key] || 0).toLocaleString()}</span>
            <label>{name}</label>
          </button>
        ))}
      </div>
      <section className="current-path">
        <div className="section-label">
          <h2>
            <Route size={17} />
            Current attack path
          </h2>
          <button className="text-button" onClick={() => w.navigate("Graph")}>
            Explore graph
            <ArrowUpRight size={13} />
          </button>
        </div>
        {path.length ? (
          <div className="path-chain">
            {path.map((id, index) => (
              <div className="path-step-wrap" key={id}>
                {index > 0 && <ArrowRight size={19} />}
                <button
                  className={`path-step ${index === path.length - 1 ? "path-destination" : ""}`}
                  onClick={() =>
                    id === "operator" ? w.navigate("Graph") : w.select(id)
                  }
                >
                  <span className="path-index">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  <strong>
                    {s.path_labels?.[id] || entityName(w.lookup, id)}
                  </strong>
                  <small>
                    {id === "operator"
                      ? "Assessment origin"
                      : w.lookup.credentials.some((c) => c.id === id)
                        ? "Discovered identity"
                        : "Confirmed access"}
                  </small>
                </button>
              </div>
            ))}
          </div>
        ) : (
          <div className="path-empty">
            <Network size={24} />
            <div>
              <strong>Build your first confirmed access path</strong>
              <p>
                Record a session or successful authentication, then pin a target
                from its inspector.
              </p>
            </div>
            <button onClick={() => w.navigate("Graph")}>
              Open graph
              <ArrowUpRight size={14} />
            </button>
          </div>
        )}
      </section>
      <div className="briefing-grid">
        <section className="attention-list">
          <div className="section-label">
            <h2>
              Suggested next checks <span>{s.opportunities.length}</span>
            </h2>
            <button
              className="text-button"
              onClick={() => w.navigate("Coverage")}
            >
              Coverage
              <ArrowUpRight size={13} />
            </button>
          </div>
          <p className="secondary small section-padding">
            Showing up to 7 suggestions from a bounded shortlist. Use
            inventories and coverage for the full assessment.
          </p>
          {s.opportunities.length ? (
            s.opportunities.slice(0, 7).map((o: Row, i: number) => (
              <div className="attention-row" key={i}>
                <div className="attention-mark" />
                <div>
                  <button onClick={() => w.select(o.entity_id)}>
                    {o.title}
                  </button>
                  <details>
                    <summary>Why this appears</summary>
                    <p>{o.why}</p>
                  </details>
                </div>
                <Badge>{o.kind}</Badge>
              </div>
            ))
          ) : (
            <p className="secondary section-padding">
              No attention items are derived from the current observations.
            </p>
          )}
        </section>
        <section className="recent-activity">
          <div className="section-label">
            <h2>Recent activity</h2>
            <button
              className="text-button"
              onClick={() => w.navigate("Timeline")}
            >
              View all
              <ArrowUpRight size={13} />
            </button>
          </div>
          {s.timeline.slice(0, 6).map((event: Row) => (
            <button
              className="activity-item"
              key={event.id}
              onClick={() =>
                event.entity_id
                  ? w.select(event.entity_id)
                  : w.navigate("Timeline")
              }
            >
              <span className="timeline-dot" />
              <div>
                <strong>{event.name}</strong>
                <span>
                  {event.source} · {time(event.created_at)}
                </span>
              </div>
            </button>
          ))}
        </section>
      </div>
      <section className="overview-assets">
        <div className="section-label">
          <h2>Environment at a glance</h2>
          <button className="text-button" onClick={() => w.navigate("Assets")}>
            All assets
            <ArrowUpRight size={13} />
          </button>
        </div>
        {assets.length ? (
          <table className="data-table">
            <thead>
              <tr>
                <th>Asset</th>
                <th>Address</th>
                <th>Operating system</th>
                <th>Reachability</th>
                <th>Access</th>
              </tr>
            </thead>
            <tbody>
              {assets.slice(0, 6).map((a) => (
                <tr key={a.id}>
                  <td>
                    <button
                      className="table-name"
                      onClick={() => w.select(a.id)}
                    >
                      {a.name}
                      <small>{a.hostname}</small>
                    </button>
                  </td>
                  <td className="mono">{a.ip || "—"}</td>
                  <td>{a.os || "Unidentified"}</td>
                  <td>
                    {s.reachability[a.id] !== undefined ? (
                      <Badge>
                        {s.reachability[a.id].length
                          ? `Via ${s.route_labels?.[s.reachability[a.id].at(-1)] || entityName(w.lookup, s.reachability[a.id].at(-1))}`
                          : "Direct"}
                      </Badge>
                    ) : (
                      <Badge>Unknown</Badge>
                    )}
                  </td>
                  <td>
                    <Badge>{a.access}</Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <Empty
            title="No assets discovered"
            detail="Import an existing scan or add your first asset."
            action={
              <button onClick={() => w.edit("assets")}>
                <Plus size={14} />
                Add manually
              </button>
            }
          />
        )}
      </section>
    </div>
  );
}
