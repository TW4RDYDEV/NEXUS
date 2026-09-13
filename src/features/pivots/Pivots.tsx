import { useState, useEffect } from "react";
import {
  Plus,
  ArrowRight,
  Network,
  GitBranch,
  Server,
  ArrowUpRight,
} from "lucide-react";
import { useWorkspace } from "../../state/workspace";
import { PageTitle, Badge, Empty } from "../../components/ui";
import { time, api, type Row } from "../../lib/api";
export function Pivots() {
  const w = useWorkspace();
  const [data, setData] = useState<Row>({ rows: [], total: 0, active: 0 });
  const [q, setQ] = useState("");
  const [page, setPage] = useState(0);
  const [error, setError] = useState("");
  useEffect(() => {
    let live = true;
    const timer = setTimeout(
      () =>
        api("pivots_page", { q, offset: page * 20 })
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
  }, [q, page, w.revision]);
  const pivots: Row[] = data.rows;
  return (
    <div className="page">
      <PageTitle
        eyebrow="NETWORK REACHABILITY"
        title="Pivots & routes"
        description="Understand how established access expands the reachable environment."
        actions={
          <button className="primary" onClick={() => w.edit("pivots")}>
            <Plus size={14} />
            Add pivot
          </button>
        }
      />
      <div className="reach-summary">
        <span>
          <strong>
            {
              Object.values(w.summary.reachability).filter(
                (r: any) => r.length === 0,
              ).length
            }
          </strong>{" "}
          direct assets
        </span>
        <span>
          <strong>
            {
              Object.values(w.summary.reachability).filter(
                (r: any) => r.length > 0,
              ).length
            }
          </strong>{" "}
          assets via pivots
        </span>
        <span>
          <strong>{data.active}</strong> active pivots
        </span>
        <span className="secondary">
          Routes require an active source session and reachable source host.
        </span>
      </div>
      <div className="table-toolbar">
        <input
          aria-label="Search pivots"
          placeholder="Search every pivot or network…"
          maxLength={200}
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setPage(0);
          }}
        />
        <span>{data.total} matching routes</span>
      </div>
      {error && (
        <div role="alert" className="error-box">
          {error}
        </div>
      )}
      {pivots.length ? (
        <div className="pivot-routes">
          {pivots.map((p) => {
            const hosts: Row[] = p.hosts;
            const source = p;
            const functional =
              p.status === "Active" &&
              source?.session_state === "Active" &&
              w.summary.reachability[p.asset_id] !== undefined;
            return (
              <section className="pivot-route" key={p.id}>
                <div className="section-label">
                  <h2>
                    <GitBranch size={17} />
                    {p.name}
                    <Badge>
                      {functional
                        ? "Reachable"
                        : p.status === "Active"
                          ? "Source unavailable"
                          : p.status}
                    </Badge>
                  </h2>
                  <button
                    className="text-button"
                    onClick={() => w.select(p.id)}
                  >
                    Inspect route
                    <ArrowUpRight size={13} />
                  </button>
                </div>
                <div className="route-diagram">
                  <button
                    className="route-node"
                    onClick={() => w.select(p.asset_id)}
                  >
                    <Server size={23} />
                    <span>Source host</span>
                    <strong>{p.source_name}</strong>
                    <small>
                      {source?.username} · {source?.privilege}
                    </small>
                  </button>
                  <ArrowRight className={functional ? "good" : ""} size={26} />
                  <button
                    className="route-node pivot"
                    onClick={() => w.select(p.id)}
                  >
                    <GitBranch size={23} />
                    <span>{p.kind}</span>
                    <strong>{p.name}</strong>
                    <small>
                      {functional ? "Route active" : "Route inactive"}
                    </small>
                  </button>
                  <ArrowRight className={functional ? "good" : ""} size={26} />
                  <div className="route-node network">
                    <Network size={23} />
                    <span>Reachable network</span>
                    <strong className="mono">{p.network}</strong>
                    <small>{p.host_total} known assets</small>
                  </div>
                </div>
                <div className="reachable-hosts">
                  <span>
                    Assets through this route{" "}
                    {p.host_total > 20
                      ? "(first 20 shown; search all assets in inventory)"
                      : ""}
                  </span>
                  {hosts.length ? (
                    hosts.map((h) => (
                      <button key={h.id} onClick={() => w.select(h.id)}>
                        {h.name}
                        <span className="mono">{h.ip}</span>
                      </button>
                    ))
                  ) : (
                    <p>
                      No discovered assets are currently reachable through this
                      pivot.
                    </p>
                  )}
                </div>
                <div className="pivot-note">
                  {p.notes || "No analyst notes recorded."}
                  <span>{time(p.created_at)}</span>
                </div>
              </section>
            );
          })}
        </div>
      ) : (
        <Empty
          title="No pivots recorded"
          detail="Add an existing tunnel and its source session to calculate downstream reachability."
          action={<button onClick={() => w.edit("pivots")}>Add pivot</button>}
        />
      )}
      <div className="paged-controls">
        <button disabled={!page} onClick={() => setPage(page - 1)}>
          Previous routes
        </button>
        <button
          disabled={(page + 1) * 20 >= data.total}
          onClick={() => setPage(page + 1)}
        >
          Next routes
        </button>
      </div>
    </div>
  );
}
