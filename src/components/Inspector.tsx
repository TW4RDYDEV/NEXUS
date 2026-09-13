import { useEffect, useState } from "react";
import {
  ArrowLeft,
  X,
  Network,
  Copy,
  Plus,
  Route,
  Pencil,
  ChevronRight,
  FileCheck,
  ArrowUpRight,
} from "lucide-react";
import {
  api,
  copy,
  saveFile,
  download,
  type Row,
  label,
  time,
  title,
} from "../lib/api";
import { isTauri } from "@tauri-apps/api/core";
import { useWorkspace, entityName } from "../state/workspace";
import { Badge, Loading } from "./ui";
import { schemas } from "../lib/schema";
export function Inspector({
  onClose,
  onBack,
  canBack,
}: {
  onClose: () => void;
  onBack: () => void;
  canBack: boolean;
}) {
  const w = useWorkspace();
  const [data, setData] = useState<Row | null>(null);
  const [tab, setTab] = useState("Overview");
  const [error, setError] = useState("");
  const [media, setMedia] = useState<Row | null>(null);
  const [alias, setAlias] = useState("");
  useEffect(() => {
    setData(null);
    setMedia(null);
    setError("");
    setTab("Overview");
    if (w.selected)
      api("inspect", { id: w.selected })
        .then(setData)
        .catch((e) => setError(e.message));
  }, [w.selected, w.revision]);
  const entity = data?.entity;
  const table = data?.table;
  const route = entity ? w.summary.reachability[entity.id] : undefined;
  async function update(field: string, value: any) {
    if (!entity || entity[field] === value) return;
    try {
      await w.run(
        "write",
        { table, data: { id: entity.id, [field]: value } },
        "Changes saved",
      );
    } catch {
      // The workspace mutation handler already presents the error to the user.
    }
  }
  return (
    <aside className="inspector">
      <div className="inspector-toolbar">
        <div>
          <button
            className="icon-button"
            disabled={!canBack}
            aria-label="Inspector back"
            onClick={onBack}
          >
            <ArrowLeft size={16} />
          </button>
          <span>INSPECTOR</span>
        </div>
        <button
          className="icon-button"
          aria-label="Close inspector"
          onClick={onClose}
        >
          <X size={16} />
        </button>
      </div>
      {error ? (
        <div className="error-box">{error}</div>
      ) : !entity ? (
        <Loading label="Loading details" />
      ) : (
        <>
          <div className="inspector-identity">
            <div className="entity-kind">
              <Network size={17} />
              {table === "assets"
                ? entity.kind
                : title(table.replace(/s$/, ""))}
            </div>
            <h2>{label(entity)}</h2>
            <div className="mono secondary">
              {entity.ip ||
                entity.username ||
                entity.url ||
                entity.network ||
                entity.hostname}
            </div>
            <div className="badge-row">
              <Badge>
                {entity.state ||
                  entity.status ||
                  entity.severity ||
                  entity.kind}
              </Badge>
              {route !== undefined && (
                <Badge>{route.length ? "Via pivot" : "Reachable"}</Badge>
              )}
              {entity.access && entity.access !== "None" && (
                <Badge>{entity.access}</Badge>
              )}
            </div>
            {route?.length > 0 && (
              <div className="route-hint">
                REACHABLE VIA{" "}
                {route
                  .map(
                    (id: string) =>
                      w.summary.route_labels?.[id] || entityName(w.lookup, id),
                  )
                  .join(" → ")}
              </div>
            )}
            <div className="inspector-quick">
              <button title="Focus in graph" onClick={() => w.focus(entity.id)}>
                <Network size={14} />
                Graph
              </button>
              {schemas[table] && (
                <button
                  title="Edit details"
                  onClick={() => w.edit(table, entity)}
                >
                  <Pencil size={13} />
                  Edit
                </button>
              )}
              {entity.ip && (
                <button
                  aria-label="Copy IP address"
                  onClick={() =>
                    copy(entity.ip)
                      .then(() => w.notify("IP address copied"))
                      .catch((e) => w.notify(e.message, true))
                  }
                >
                  <Copy size={13} />
                </button>
              )}
            </div>
          </div>
          <div className="inspector-tabs">
            {["Overview", "Provenance"].map((t) => (
              <button
                key={t}
                className={tab === t ? "active" : ""}
                onClick={() => setTab(t)}
              >
                {t}
              </button>
            ))}
          </div>
          <div className="inspector-body">
            {tab === "Provenance" ? (
              <>
                <p className="secondary small">
                  Every source remains available when observations disagree.
                </p>
                {data?.provenance.length ? (
                  data.provenance.map((p: Row) => (
                    <div className="provenance-item" key={p.id}>
                      <div>
                        <strong>{title(p.field)}</strong>
                        <Badge>{p.conflict ? "Conflict" : p.confidence}</Badge>
                      </div>
                      <pre>{p.value}</pre>
                      <span>
                        {p.source} · {p.source_file || "Manual observation"}
                      </span>
                      <small>{time(p.created_at)}</small>
                    </div>
                  ))
                ) : (
                  <p className="secondary">
                    No imported observations are linked to this record.
                  </p>
                )}
              </>
            ) : (
              <>
                {table === "assets" && (
                  <>
                    <div className="inspector-section">
                      <h3>Host details</h3>
                      <dl>
                        <dt>Operating system</dt>
                        <dd>{entity.os || "Unidentified"}</dd>
                        <dt>Hostname</dt>
                        <dd className="mono">
                          {entity.hostname || "Unrecorded"}
                        </dd>
                        <dt>First seen</dt>
                        <dd>{time(entity.first_seen)}</dd>
                        <dt>Last seen</dt>
                        <dd>{time(entity.last_seen)}</dd>
                      </dl>
                      {data?.aliases?.length > 0 && (
                        <div className="alias-list">
                          {data.aliases.map((a: Row) => (
                            <span className="mono" key={a.alias}>
                              {a.alias}
                            </span>
                          ))}
                        </div>
                      )}
                      <form
                        className="alias-form"
                        onSubmit={(e) => {
                          e.preventDefault();
                          w.run(
                            "alias",
                            { asset_id: entity.id, alias },
                            "Alias recorded",
                          )
                            .then(() => setAlias(""))
                            .catch(() => {});
                        }}
                      >
                        <input
                          aria-label="Add asset alias"
                          placeholder="Additional IP or hostname"
                          value={alias}
                          onChange={(e) => setAlias(e.target.value)}
                          required
                        />
                        <button aria-label="Record alias" title="Record alias">
                          <Plus size={13} />
                        </button>
                      </form>
                      <label className="inline-checkbox">
                        <input
                          type="checkbox"
                          checked={entity.direct === 1}
                          onChange={(e) =>
                            update("direct", e.target.checked ? 1 : 0)
                          }
                        />
                        Directly reachable
                      </label>
                    </div>
                    <div className="inspector-section">
                      <div className="section-label">
                        <h3>Confirmed access path</h3>
                        <Route size={14} />
                      </div>
                      <button
                        className="full-width"
                        onClick={() =>
                          w
                            .run(
                              "path",
                              { target: entity.id, pin: true },
                              "Preferred path pinned",
                            )
                            .then(() => w.navigate("Graph"))
                            .catch(() => {})
                        }
                      >
                        Calculate and pin path
                        <ArrowUpRight size={14} />
                      </button>
                    </div>
                  </>
                )}
                {table === "services" && (
                  <div className="inspector-section">
                    <h3>Service observation</h3>
                    <dl>
                      {[
                        "protocol",
                        "port",
                        "product",
                        "version",
                        "tls",
                        "url",
                        "title",
                        "banner",
                      ].map((f) => (
                        <div className="dl-row" key={f}>
                          <dt>{title(f)}</dt>
                          <dd>
                            {f === "tls"
                              ? entity[f]
                                ? "Observed"
                                : "Unrecorded"
                              : entity[f] || "Unrecorded"}
                          </dd>
                        </div>
                      ))}
                    </dl>
                    <button
                      onClick={() => w.select(entity.asset_id)}
                      className="inline-link"
                    >
                      {entity.reference_labels?.asset_id ||
                        entityName(w.lookup, entity.asset_id)}
                      <ChevronRight size={13} />
                    </button>
                  </div>
                )}
                {[
                  "sessions",
                  "pivots",
                  "findings",
                  "credentials",
                  "credential_tests",
                ].includes(table) && (
                  <div className="inspector-section">
                    <h3>Record details</h3>
                    <dl>
                      {Object.entries(entity)
                        .filter(
                          ([k, v]) =>
                            ![
                              "id",
                              "name",
                              "notes",
                              "ciphertext",
                              "reference_labels",
                              "created_at",
                              "first_seen",
                              "last_seen",
                            ].includes(k) &&
                            v !== null &&
                            v !== "",
                        )
                        .map(([key, value]) => (
                          <div key={key} className="dl-row">
                            <dt>{title(key)}</dt>
                            <dd>
                              {key.endsWith("_id") ? (
                                <button
                                  className="text-button"
                                  onClick={() => w.select(String(value))}
                                >
                                  {entity.reference_labels?.[key] ||
                                    entityName(w.lookup, String(value))}
                                </button>
                              ) : (
                                String(value)
                              )}
                            </dd>
                          </div>
                        ))}
                    </dl>
                  </div>
                )}
                {table === "credentials" && (
                  <button
                    className="full-width"
                    onClick={() =>
                      w.edit("credential_tests", { credential_id: entity.id })
                    }
                  >
                    Record authentication test
                    <Plus size={14} />
                  </button>
                )}
                {table === "evidence" && (
                  <div className="inspector-section">
                    <h3>Evidence content</h3>
                    <button
                      className="text-button"
                      onClick={async () => {
                        try {
                          if (isTauri()) {
                            const path = await saveFile(
                              entity.name +
                                (entity.kind === "Screenshot"
                                  ? ".png"
                                  : ".txt"),
                            );
                            if (path) {
                              await api("evidence_export", {
                                id: entity.id,
                                path,
                              });
                              w.notify("Evidence exported");
                            }
                          } else {
                            const item = await api("evidence_read", {
                              id: entity.id,
                            });
                            if (item.base64) {
                              const bytes = Uint8Array.from(
                                atob(item.base64),
                                (c) => c.charCodeAt(0),
                              );
                              const url = URL.createObjectURL(
                                new Blob([bytes], {
                                  type: item.mime || "application/octet-stream",
                                }),
                              );
                              const link = document.createElement("a");
                              link.href = url;
                              link.download =
                                entity.name +
                                (item.mime === "image/png"
                                  ? ".png"
                                  : item.mime === "image/jpeg"
                                    ? ".jpg"
                                    : ".bin");
                              link.click();
                              setTimeout(() => URL.revokeObjectURL(url), 1000);
                            } else download(entity.name + ".txt", item.text);
                          }
                        } catch (e) {
                          w.notify(
                            e instanceof Error
                              ? e.message
                              : "Evidence export failed",
                            true,
                          );
                        }
                      }}
                    >
                      Export evidence
                    </button>
                    <pre className="evidence-text">{entity.body}</pre>
                    {entity.file_name && (
                      <>
                        <button
                          onClick={() =>
                            api("evidence_read", { id: entity.id })
                              .then(setMedia)
                              .catch((e) => w.notify(e.message, true))
                          }
                        >
                          <FileCheck size={14} />
                          Verify and view attachment
                        </button>
                        {media?.mime ? (
                          <img
                            className="evidence-image"
                            src={`data:${media.mime};base64,${media.base64}`}
                            alt={entity.name}
                          />
                        ) : (
                          media?.text && (
                            <pre className="evidence-text">{media.text}</pre>
                          )
                        )}
                        <p className="hash small mono">{entity.sha256}</p>
                      </>
                    )}
                  </div>
                )}
                {[
                  "services",
                  "sessions",
                  "pivots",
                  "credentials",
                  "credential_tests",
                  "findings",
                  "evidence",
                ].map((t) => {
                  const related =
                    data?.related.filter((r: Row) => r.table === t) || [];
                  if (
                    !related.length &&
                    !(
                      table === "assets" &&
                      ["services", "sessions", "findings", "evidence"].includes(
                        t,
                      )
                    )
                  )
                    return null;
                  return (
                    <div key={t} className="inspector-section">
                      <div className="section-label">
                        <h3>
                          {title(t)} <span>{related.length}</span>
                        </h3>
                        {table === "assets" && (
                          <button
                            className="icon-button"
                            title={`Add ${t}`}
                            onClick={() =>
                              w.edit(
                                t,
                                t === "evidence"
                                  ? { entity_id: entity.id }
                                  : t === "credentials"
                                    ? { source_id: entity.id }
                                    : { asset_id: entity.id },
                              )
                            }
                          >
                            <Plus size={14} />
                          </button>
                        )}
                      </div>
                      {related.map((r: Row) => (
                        <button
                          className="related-row"
                          key={r.entity.id}
                          onClick={() => w.select(r.entity.id)}
                        >
                          <div>
                            <strong>
                              {label(r.entity)}
                              {t === "services" ? ` :${r.entity.port}` : ""}
                            </strong>
                            <span>
                              {r.entity.product ||
                                r.entity.username ||
                                r.entity.network ||
                                r.entity.kind}
                            </span>
                          </div>
                          {(r.entity.severity ||
                            r.entity.result ||
                            r.entity.state) && (
                            <Badge>
                              {r.entity.severity ||
                                r.entity.result ||
                                r.entity.state}
                            </Badge>
                          )}
                          <ChevronRight size={13} />
                        </button>
                      ))}
                    </div>
                  );
                })}
                {"notes" in entity && (
                  <div className="inspector-section">
                    <h3>
                      Analyst notes <span>Autosaved on blur</span>
                    </h3>
                    <textarea
                      aria-label="Analyst notes"
                      defaultValue={entity.notes}
                      key={entity.id + entity.notes}
                      onBlur={(e) => update("notes", e.target.value)}
                      rows={5}
                    />
                  </div>
                )}
              </>
            )}
          </div>
          <div className="inspector-footer">
            <span className="status-dot" />
            Stored locally · {time(entity.created_at || entity.first_seen)}
          </div>
        </>
      )}
    </aside>
  );
}
