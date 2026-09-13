import { useEffect, useState } from "react";
import { Search, ChevronRight } from "lucide-react";
import { useWorkspace, entityName } from "../../state/workspace";
import { PageTitle, Badge, Empty } from "../../components/ui";
import { api, type Row } from "../../lib/api";
export function Coverage() {
  const w = useWorkspace();
  const [q, setQ] = useState("");
  const [filter, setFilter] = useState("");
  const coverage = w.summary.coverage;
  const [rows, setRows] = useState<Row[]>([]);
  const [total, setTotal] = useState(0);
  const [limit, setLimit] = useState(100);
  useEffect(() => {
    let current = true;
    const timer = setTimeout(
      () =>
        api("coverage", { q, state: filter, offset: limit - 100, limit: 100 })
          .then((data) => {
            if (current) {
              setRows(data.rows);
              setTotal(data.total);
            }
          })
          .catch((e) => w.notify(e.message, true)),
      120,
    );
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [q, filter, limit, w.revision]);
  return (
    <div className="page">
      <PageTitle
        eyebrow="METHODOLOGY & GAPS"
        title="Assessment coverage"
        description="Record what was checked. Understand what remains unknown."
      />
      <div className="coverage-summary">
        <div>
          <strong>
            {coverage.percent === null ? "—" : `${coverage.percent}%`}
          </strong>
          <span>
            {coverage.complete} complete / {coverage.applicable} applicable
            checks
          </span>
        </div>
        <div className="coverage-bar">
          <span style={{ width: `${coverage.percent || 0}%` }} />
        </div>
        <p>
          Partial checks remain in the denominator. Not Applicable checks are
          excluded.
        </p>
      </div>
      <div className="table-toolbar">
        <label className="search-input">
          <Search size={15} />
          <input
            aria-label="Search coverage"
            placeholder=" "
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setLimit(100);
            }}
          />
          <span>Search checks or services</span>
        </label>
        <select
          aria-label="Coverage state filter"
          value={filter}
          onChange={(e) => {
            setFilter(e.target.value);
            setLimit(100);
          }}
        >
          <option value="">All states</option>
          {["Complete", "Partial", "Untested", "Not Applicable"].map((s) => (
            <option key={s}>{s}</option>
          ))}
        </select>
      </div>
      {rows.length ? (
        <table className="data-table coverage-table">
          <thead>
            <tr>
              <th>Host / service</th>
              <th>Methodology check</th>
              <th>State</th>
              <th>Source</th>
            </tr>
          </thead>
          <tbody>
            {rows.slice(0, limit).map((r: Row) => (
              <tr key={`${r.service_id}-${r.definition_id}`}>
                <td>
                  <button
                    className="table-name"
                    onClick={() => w.select(r.service_id)}
                  >
                    {r.asset_name || entityName(w.lookup, r.asset_id)}
                    <small>{r.service}</small>
                  </button>
                </td>
                <td>{r.name}</td>
                <td>
                  <select
                    aria-label={`${r.service} ${r.name}`}
                    value={r.state}
                    onChange={(e) =>
                      w
                        .run(
                          "write",
                          {
                            table: "coverage_results",
                            data: {
                              ...(r.id ? { id: r.id } : {}),
                              service_id: r.service_id,
                              definition_id: r.definition_id,
                              state: e.target.value,
                              source: "Manual",
                              notes: r.notes,
                            },
                          },
                          "Coverage observation recorded",
                        )
                        .catch(() => {})
                    }
                  >
                    {["Complete", "Partial", "Untested", "Not Applicable"].map(
                      (s) => (
                        <option key={s}>{s}</option>
                      ),
                    )}
                  </select>
                </td>
                <td className="secondary">{r.source || "Not tested"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <Empty
          title="No matching coverage checks"
          detail="Import or add a service to initialize its methodology checklist."
        />
      )}
      <div className="pagination">
        <span>
          {Math.min(limit - 99, total)}–{Math.min(limit, total)} of {total}{" "}
          checks
        </span>
        <div>
          <button disabled={limit <= 100} onClick={() => setLimit(limit - 100)}>
            Previous 100
          </button>
          <button
            disabled={limit >= total}
            onClick={() => setLimit(limit + 100)}
          >
            Next 100
          </button>
        </div>
      </div>
      <section className="opportunities-section">
        <div className="section-label">
          <h2>
            Untested opportunities <span>{w.summary.opportunities.length}</span>
          </h2>
        </div>
        {w.summary.opportunities.slice(0, 100).map((o: Row, i: number) => (
          <details className="opportunity-detail" key={i}>
            <summary>
              <Badge>{o.kind}</Badge>
              {o.title}
            </summary>
            <p>{o.why}</p>
            <button
              className="text-button"
              onClick={() => w.select(o.entity_id)}
            >
              Inspect related entity
              <ChevronRight size={13} />
            </button>
          </details>
        ))}
      </section>
    </div>
  );
}
