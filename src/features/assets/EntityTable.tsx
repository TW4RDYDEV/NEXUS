import { useEffect, useState } from "react";
import {
  Plus,
  Search,
  ChevronLeft,
  ChevronRight,
  ArrowDownUp,
  Archive,
  MoreHorizontal,
  Download,
  X,
} from "lucide-react";
import {
  api,
  copy,
  download,
  type Row,
  label,
  time,
  title,
} from "../../lib/api";
import { useWorkspace, entityName } from "../../state/workspace";
import { PageTitle, Badge, Empty, Loading, Modal } from "../../components/ui";
import { singular } from "../../lib/schema";
const columns: Record<string, string[]> = {
  assets: ["name", "ip", "os", "services", "reachability", "access", "tags"],
  services: [
    "name",
    "asset_id",
    "port",
    "protocol",
    "product",
    "version",
    "status",
  ],
  sessions: [
    "name",
    "asset_id",
    "username",
    "privilege",
    "kind",
    "state",
    "last_active",
  ],
  pivots: ["name", "asset_id", "network", "kind", "status"],
  findings: [
    "name",
    "severity",
    "asset_id",
    "status",
    "evidence",
    "created_at",
  ],
  evidence: ["name", "kind", "entity_id", "created_at"],
  credentials: ["username", "context", "kind", "source", "created_at"],
  timeline_events: ["name", "kind", "source", "created_at"],
};
const subtitles: Record<string, string> = {
  assets: "An inventory of discovered systems and their observed state.",
  services: "Normalized service observations across the engagement.",
  sessions: "Track established access and its current privilege.",
  findings: "Turn validated observations into evidence-backed findings.",
  evidence: "A traceable record of what you observed.",
  timeline_events: "The chronological record of this engagement.",
};
export function EntityTable({ table = "assets" }: { table?: string }) {
  const w = useWorkspace();
  const [q, setQ] = useState("");
  const [rows, setRows] = useState<Row[]>([]);
  const [total, setTotal] = useState(0);
  const [offset, setOffset] = useState(0);
  const [sort, setSort] = useState("name");
  const [descending, setDescending] = useState(false);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<Row>({});
  const [selected, setSelected] = useState<string[]>([]);
  const [context, setContext] = useState<{
    row: Row;
    x: number;
    y: number;
  } | null>(null);
  const [archive, setArchive] = useState(false);
  useEffect(() => {
    setQ("");
    setFilter({});
    setOffset(0);
    setSelected([]);
    setSort("name");
  }, [table]);
  useEffect(() => {
    let active = true;
    setLoading(true);
    const timer = setTimeout(
      () =>
        api("list", {
          table,
          q,
          offset,
          limit: 50,
          sort,
          descending,
          filters: filter,
        })
          .then((d) => {
            if (active) {
              setRows(d.rows);
              setTotal(d.total);
              setLoading(false);
            }
          })
          .catch((e) => {
            if (active) {
              setLoading(false);
              w.notify(e.message, true);
            }
          }),
      120,
    );
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [table, q, offset, sort, descending, filter, w.revision]);
  const display = (row: Row, key: string) => {
    if (key === "services") {
      const services: Row[] = row.observed_services || [];
      return (
        <span className="mono services-inline">
          {services
            .map((s) => s.port)
            .sort((a, b) => a - b)
            .join(" · ") || "Unidentified"}
          {row.service_count > 12 ? " · +" + (row.service_count - 12) : ""}
        </span>
      );
    }
    if (key === "reachability") {
      const route = w.summary.reachability[row.id];
      return route === undefined ? (
        <Badge>Unknown</Badge>
      ) : (
        <Badge>
          {route.length
            ? `Via ${w.summary.route_labels?.[route[route.length - 1]] || entityName(w.lookup, route[route.length - 1])}`
            : "Direct"}
        </Badge>
      );
    }
    if (key === "evidence") {
      const count = row.evidence_count || 0;
      return (
        <span className={count ? "secondary" : "attention"}>
          {count} {count === 1 ? "item" : "items"}
        </span>
      );
    }
    if (key.endsWith("_id"))
      return (
        <span>
          {row.reference_labels?.[key] || entityName(w.lookup, row[key])}
        </span>
      );
    if (["status", "severity", "state", "access", "privilege"].includes(key))
      return <Badge>{row[key] || "Unknown"}</Badge>;
    if (key.endsWith("_at") || key === "last_active")
      return <span className="secondary small">{time(row[key])}</span>;
    if (key === "name" || key === "username")
      return (
        <button className="table-name" onClick={() => w.select(row.id)}>
          {label(row)}
          {table === "assets" && <small>{row.hostname || row.kind}</small>}
        </button>
      );
    return (
      <span
        className={
          ["ip", "port", "protocol", "context"].includes(key) ? "mono" : ""
        }
      >
        {(key === "kind" ? row.kind_label || row[key] : row[key]) || "—"}
      </span>
    );
  };
  return (
    <div className="page">
      <PageTitle
        eyebrow="ENGAGEMENT INVENTORY"
        title={table === "timeline_events" ? "Timeline" : title(table)}
        description={subtitles[table]}
        actions={
          <>
            {table === "assets" && (
              <button onClick={w.importFile}>Import scan</button>
            )}
            {table === "findings" && (
              <button
                onClick={() =>
                  w
                    .run(
                      "export_report",
                      {},
                      "Report saved to engagement exports",
                    )
                    .then((r) => download("NEXUS-report.md", r.text))
                    .catch(() => {})
                }
              >
                <Download size={14} />
                Export report
              </button>
            )}
            {table !== "timeline_events" && (
              <button className="primary" onClick={() => w.edit(table)}>
                <Plus size={15} />
                Add {singular[table]}
              </button>
            )}
          </>
        }
      />
      <div className="table-toolbar">
        <label className="search-input">
          <Search size={15} />
          <input
            aria-label={`Search ${table}`}
            placeholder=" "
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setOffset(0);
            }}
          />
          <span>Search {table === "timeline_events" ? "activity" : table}</span>
        </label>
        <div className="filters">
          {table === "assets" && (
            <>
              <select
                aria-label="Asset type filter"
                value={filter.kind || ""}
                onChange={(e) => {
                  setFilter({ ...filter, kind: e.target.value });
                  if (!e.target.value)
                    setFilter((f) => {
                      const n = { ...f };
                      delete n.kind;
                      return n;
                    });
                  setOffset(0);
                }}
              >
                <option value="">All asset types</option>
                {["Host", "Network", "Domain", "Web Application", "User"].map(
                  (o) => (
                    <option key={o}>{o}</option>
                  ),
                )}
              </select>
              <select
                aria-label="Access filter"
                value={filter.access || ""}
                onChange={(e) => {
                  const next = { ...filter };
                  if (e.target.value) next.access = e.target.value;
                  else delete next.access;
                  setFilter(next);
                  setOffset(0);
                }}
              >
                <option value="">Any access</option>
                {["None", "User", "Administrator", "root", "Unknown"].map(
                  (o) => (
                    <option key={o}>{o}</option>
                  ),
                )}
              </select>
            </>
          )}
          {table === "findings" && (
            <select
              aria-label="Severity filter"
              value={filter.severity || ""}
              onChange={(e) => {
                setFilter(e.target.value ? { severity: e.target.value } : {});
                setOffset(0);
              }}
            >
              <option value="">All severities</option>
              {["Critical", "High", "Medium", "Low", "Info"].map((o) => (
                <option key={o}>{o}</option>
              ))}
            </select>
          )}
          {table === "sessions" && (
            <select
              aria-label="Session state filter"
              value={filter.state || ""}
              onChange={(e) => {
                setFilter(e.target.value ? { state: e.target.value } : {});
                setOffset(0);
              }}
            >
              <option value="">All states</option>
              {["Active", "Stale", "Closed", "Unknown"].map((o) => (
                <option key={o}>{o}</option>
              ))}
            </select>
          )}
          {selected.length > 0 && (
            <button onClick={() => setArchive(true)}>
              <Archive size={14} />
              Archive {selected.length}
            </button>
          )}
        </div>
      </div>
      {Object.keys(filter).length > 0 && (
        <div className="filter-chips">
          {Object.entries(filter).map(([key, value]) => (
            <button
              key={key}
              onClick={() =>
                setFilter((f) => {
                  const n = { ...f };
                  delete n[key];
                  return n;
                })
              }
            >
              {title(key)}: {String(value)}
              <X size={12} />
            </button>
          ))}
        </div>
      )}
      <div className="table-scroll">
        {loading && rows.length === 0 ? (
          <Loading />
        ) : rows.length === 0 ? (
          <Empty
            title={
              q || Object.keys(filter).length
                ? "No matching records"
                : `No ${table === "timeline_events" ? "activity recorded" : table + " recorded"}`
            }
            detail={
              q
                ? "Adjust the search or remove a filter."
                : table === "assets"
                  ? "Import a discovery scan or add the first asset."
                  : "Add an observation to begin building the engagement record."
            }
            action={
              table !== "timeline_events" && (
                <button onClick={() => w.edit(table)}>
                  <Plus size={14} />
                  Add {singular[table]}
                </button>
              )
            }
          />
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                {table === "assets" && (
                  <th className="check-cell">
                    <input
                      aria-label="Select visible assets"
                      type="checkbox"
                      checked={
                        rows.length > 0 &&
                        rows.every((r) => selected.includes(r.id))
                      }
                      onChange={(e) =>
                        setSelected(
                          e.target.checked ? rows.map((r) => r.id) : [],
                        )
                      }
                    />
                  </th>
                )}
                {(columns[table] || ["name"]).map((key) => (
                  <th key={key}>
                    {["services", "reachability", "evidence"].includes(key) ? (
                      title(key)
                    ) : (
                      <button
                        onClick={() => {
                          setSort(key);
                          setDescending(sort === key ? !descending : false);
                        }}
                      >
                        {title(key.replace("_id", ""))}
                        {sort === key && <ArrowDownUp size={11} />}
                      </button>
                    )}
                  </th>
                ))}
                {table !== "timeline_events" && <th />}
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr
                  key={row.id}
                  className={w.selected === row.id ? "selected" : ""}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    if (table !== "timeline_events")
                      setContext({ row, x: e.clientX, y: e.clientY });
                  }}
                  onDoubleClick={() => w.select(row.id)}
                >
                  {table === "assets" && (
                    <td className="check-cell">
                      <input
                        type="checkbox"
                        aria-label={`Select ${row.name}`}
                        checked={selected.includes(row.id)}
                        onChange={(e) =>
                          setSelected(
                            e.target.checked
                              ? [...selected, row.id]
                              : selected.filter((i) => i !== row.id),
                          )
                        }
                      />
                    </td>
                  )}
                  {(columns[table] || ["name"]).map((key) => (
                    <td key={key}>
                      {table === "timeline_events" && key === "name" ? (
                        <button
                          className="table-name"
                          onClick={() =>
                            row.entity_id
                              ? w.select(row.entity_id)
                              : w.notify("This event applies to the engagement")
                          }
                        >
                          {row.name}
                        </button>
                      ) : (
                        display(row, key)
                      )}
                    </td>
                  ))}
                  {table !== "timeline_events" && (
                    <td>
                      <button
                        className="icon-button"
                        aria-label={`Actions for ${label(row)}`}
                        onClick={(e) =>
                          setContext({ row, x: e.clientX, y: e.clientY })
                        }
                      >
                        <MoreHorizontal size={16} />
                      </button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
      <div className="pagination">
        <span>
          {total
            ? `${offset + 1}–${Math.min(offset + 50, total)} of ${total.toLocaleString()}`
            : "0"}{" "}
          records
        </span>
        <div>
          <button
            aria-label="Previous page"
            disabled={offset === 0}
            onClick={() => setOffset(Math.max(0, offset - 50))}
          >
            <ChevronLeft size={15} />
          </button>
          <span>Page {Math.floor(offset / 50) + 1}</span>
          <button
            aria-label="Next page"
            disabled={offset + 50 >= total}
            onClick={() => setOffset(offset + 50)}
          >
            <ChevronRight size={15} />
          </button>
        </div>
      </div>
      {context && (
        <>
          <div className="menu-dismiss" onClick={() => setContext(null)} />
          <div
            className="context-menu"
            style={{
              left: Math.min(context.x, window.innerWidth - 230),
              top: Math.min(context.y, window.innerHeight - 350),
            }}
          >
            {[
              ["Open inspector", () => w.select(context.row.id)],
              ["Focus in graph", () => w.focus(context.row.id)],
              ["Edit record", () => w.edit(table, context.row)],
              ...(table === "assets"
                ? [
                    [
                      "Add credential",
                      () =>
                        w.edit("credentials", { source_id: context.row.id }),
                    ],
                    [
                      "Add session",
                      () => w.edit("sessions", { asset_id: context.row.id }),
                    ],
                    [
                      "Add finding",
                      () => w.edit("findings", { asset_id: context.row.id }),
                    ],
                    [
                      "Add evidence",
                      () => w.edit("evidence", { entity_id: context.row.id }),
                    ],
                    [
                      "Copy IP",
                      () =>
                        copy(context.row.ip || "").then(() =>
                          w.notify("IP copied"),
                        ),
                    ],
                    [
                      "Copy hostname",
                      () =>
                        copy(context.row.hostname || "").then(() =>
                          w.notify("Hostname copied"),
                        ),
                    ],
                    [
                      "Archive asset",
                      () => {
                        setSelected([context.row.id]);
                        setArchive(true);
                      },
                    ],
                  ]
                : []),
            ].map(([name, action]) => (
              <button
                key={String(name)}
                onClick={() => {
                  (action as () => void)();
                  setContext(null);
                }}
              >
                {String(name)}
              </button>
            ))}
          </div>
        </>
      )}
      {archive && (
        <Modal
          title="Archive selected assets?"
          subtitle="Archived assets remain in the database and audit history. Undo restores the most recent manual change."
          onClose={() => setArchive(false)}
        >
          <p>
            {selected.length} assets will be removed from the active inventory.
          </p>
          <footer>
            <button onClick={() => setArchive(false)}>Cancel</button>
            <button
              className="danger"
              onClick={() =>
                w
                  .run(
                    "archive",
                    { ids: selected, confirmed: true },
                    "Assets archived",
                  )
                  .then(() => {
                    setArchive(false);
                    setSelected([]);
                  })
                  .catch(() => {})
              }
            >
              Archive assets
            </button>
          </footer>
        </Modal>
      )}
    </div>
  );
}
