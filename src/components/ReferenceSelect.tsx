import { useEffect, useState } from "react";
import { api, type Row } from "../lib/api";

export function ReferenceSelect({
  table,
  label,
  value,
  required,
  parentId,
  onChange,
}: {
  table: string;
  label: string;
  value: string;
  required?: boolean;
  parentId?: string;
  onChange: (id: string, row?: Row) => void;
}) {
  const [q, setQ] = useState("");
  const [offset, setOffset] = useState(0);
  const [page, setPage] = useState<Row>({ rows: [], total: 0, selected: null });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    setBusy(true);
    setError("");
    const timer = setTimeout(
      () =>
        api("references", {
          table,
          q,
          offset,
          limit: 50,
          selected_id: value,
          asset_id: parentId || "",
        })
          .then((result) => {
            if (active) setPage(result);
          })
          .catch((e) => {
            if (active) setError(e.message);
          })
          .finally(() => {
            if (active) setBusy(false);
          }),
      q ? 180 : 0,
    );
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [table, q, offset, value, parentId]);
  useEffect(() => setOffset(0), [parentId, table]);
  const rows: Row[] = [
    ...(page.selected && !page.rows.some((r: Row) => r.id === page.selected.id)
      ? [page.selected]
      : []),
    ...page.rows,
  ];
  return (
    <span className="reference-control">
      <input
        aria-label={`Search ${label} references`}
        value={q}
        maxLength={200}
        placeholder="Search the entire engagement…"
        onChange={(e) => {
          setQ(e.target.value);
          setOffset(0);
        }}
      />
      <select
        aria-label={label}
        required={required}
        value={value || ""}
        onChange={(e) =>
          onChange(
            e.target.value,
            rows.find((r) => r.id === e.target.value),
          )
        }
      >
        <option value="">{required ? "Select an entity" : "None"}</option>
        {value && !rows.some((r) => r.id === value) && (
          <option value={value}>
            {busy ? "Loading selected entity…" : value}
          </option>
        )}
        {rows.map((row) => (
          <option key={row.id} value={row.id}>
            {row.name}
            {row.detail ? ` · ${row.detail}` : ""}
            {table === "all" ? ` [${row.collection}]` : ""}
          </option>
        ))}
      </select>
      <span className="reference-pagination">
        <button
          type="button"
          aria-label={`Previous ${label} references`}
          disabled={busy || offset === 0}
          onClick={() => setOffset(Math.max(0, offset - 50))}
        >
          Previous
        </button>
        <span role="status">
          {busy
            ? "Searching…"
            : `${page.total ? offset + 1 : 0}–${Math.min(offset + 50, page.total)} of ${page.total}`}
        </span>
        <button
          type="button"
          aria-label={`Next ${label} references`}
          disabled={busy || offset + 50 >= page.total}
          onClick={() => setOffset(offset + 50)}
        >
          Next
        </button>
      </span>
      {error && (
        <span className="bad" role="alert">
          {error}
        </span>
      )}
    </span>
  );
}
