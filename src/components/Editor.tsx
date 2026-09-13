import { ReferenceSelect } from "./ReferenceSelect";
import { useState, useRef } from "react";
import { Paperclip } from "lucide-react";
import { Modal } from "./ui";
import { schemas, singular, type Field } from "../lib/schema";
import { type Row } from "../lib/api";
import { useWorkspace, type EditorSpec } from "../state/workspace";
export function Editor({
  spec,
  onClose,
}: {
  spec: EditorSpec;
  onClose: () => void;
}) {
  const w = useWorkspace();
  const fields = schemas[spec.table].map((field) =>
    field.vocabulary === "relationships"
      ? {
          ...field,
          options: (w.summary.relationship_vocabulary as Row[])
            .filter((r) => r.id !== "NX_AUTHENTICATES_TO")
            .map((r) => r.id),
          default: "NX_CONNECTS_TO",
        }
      : field,
  );
  const initial = Object.fromEntries(
    fields.map((f) => [
      f.key,
      spec.data?.[f.key] ?? f.default ?? (f.type === "reference" ? null : ""),
    ]),
  );
  const [data, setData] = useState<Row>(initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [attachment, setAttachment] = useState<File | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const set = (key: string, value: any) => {
    setData((d) => {
      const next = { ...d, [key]: value };
      if (key === "session_id" && spec.table === "pivots") {
        const session = w.lookup.sessions.find((s) => s.id === value);
        if (session) next.asset_id = session.asset_id;
      }
      if (key === "asset_id" && d.asset_id !== value && spec.table !== "pivots")
        next.service_id = null;
      return next;
    });
  };
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      const payload = { ...data };
      for (const f of fields) {
        if (f.type === "number") payload[f.key] = Number(payload[f.key]);
        if (f.type === "reference" && !payload[f.key]) payload[f.key] = null;
      }
      if (spec.table === "credentials") {
        await w.run(
          "credential",
          { ...payload, ...(spec.data?.id ? { id: spec.data.id } : {}) },
          "Credential encrypted and added",
        );
      } else if (spec.table === "evidence" && !spec.data?.id) {
        if (attachment) {
          if (attachment.size > 20 * 1024 * 1024)
            throw Error("Evidence exceeds 20 MiB");
          payload.base64 = await new Promise<string>((resolve, reject) => {
            const r = new FileReader();
            r.onload = () => resolve(String(r.result).split(",")[1]);
            r.onerror = reject;
            r.readAsDataURL(attachment);
          });
        }
        await w.run("evidence_attach", payload, "Evidence attached");
      } else {
        await w.run(
          "write",
          {
            table: spec.table,
            data: {
              ...payload,
              ...(spec.data?.id ? { id: spec.data.id } : {}),
            },
          },
          `${singular[spec.table]} ${spec.data?.id ? "updated" : "added"}`,
        );
      }
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to write record");
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal
      title={`${spec.data?.id ? "Edit" : "Add"} ${singular[spec.table]}`}
      subtitle={
        spec.table === "credential_tests"
          ? "Record an observed authentication result. This does not execute a login attempt."
          : spec.table === "pivots"
            ? "Model an existing tunnel. Reachability is recalculated from its live source session."
            : "Changes are persisted in this engagement."
      }
      onClose={onClose}
      wide={fields.length > 8}
    >
      <form onSubmit={submit}>
        <div className="form-grid">
          {fields.map((f) => (
            <FieldInput
              key={f.key}
              field={f}
              value={data[f.key]}
              onChange={(value) => set(f.key, value)}
              onParentChange={(id) => set("asset_id", id)}
              data={data}
            />
          ))}
        </div>
        {spec.table === "evidence" && !spec.data?.id && (
          <div className="attachment-picker">
            <input
              ref={fileInput}
              type="file"
              hidden
              onChange={(e) => setAttachment(e.target.files?.[0] ?? null)}
            />
            <button type="button" onClick={() => fileInput.current?.click()}>
              <Paperclip size={15} />
              {attachment ? attachment.name : "Attach a file"}
            </button>
            <span>20 MiB maximum · retained with SHA-256</span>
          </div>
        )}
        {error && (
          <div role="alert" className="error-box">
            {error}
          </div>
        )}
        <footer>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button className="primary" disabled={busy}>
            {busy
              ? "Writing…"
              : spec.data?.id
                ? "Apply changes"
                : `Add ${singular[spec.table]}`}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
function FieldInput({
  field: f,
  value,
  onChange,
  data,
  onParentChange,
}: {
  field: Field;
  value: any;
  onChange: (v: any) => void;
  data: Row;
  onParentChange: (id: string) => void;
}) {
  const w = useWorkspace();
  const props = {
    id: `field-${f.key}`,
    "aria-label": f.label,
    required: f.required,
    value: value ?? "",
    onChange: (
      e: React.ChangeEvent<
        HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
      >,
    ) => onChange(e.target.value),
  };
  return (
    <label
      className={`${f.wide ? "wide-field" : ""} ${f.type === "checkbox" ? "checkbox-label" : ""}`}
      htmlFor={props.id}
    >
      <span>
        {f.label}
        {f.required && <i aria-hidden="true"> *</i>}
      </span>
      {f.type === "textarea" ? (
        <textarea {...props} rows={4} />
      ) : f.type === "select" ? (
        <select {...props}>
          {f.options?.map((o) => (
            <option key={o} value={o}>
              {f.vocabulary
                ? w.summary.relationship_vocabulary.find((r: Row) => r.id === o)
                    ?.label || o
                : o}
            </option>
          ))}
        </select>
      ) : f.type === "reference" ? (
        <ReferenceSelect
          table={f.table || "all"}
          label={f.label}
          value={value || ""}
          required={f.required}
          parentId={f.key === "service_id" ? data.asset_id : undefined}
          onChange={(id, row) => {
            onChange(id);
            if (f.key === "session_id" && row?.asset_id)
              onParentChange(row.asset_id);
          }}
        />
      ) : f.type === "checkbox" ? (
        <input
          id={props.id}
          type="checkbox"
          checked={Boolean(value)}
          onChange={(e) => onChange(e.target.checked ? 1 : 0)}
        />
      ) : (
        <input
          {...props}
          autoComplete={f.type === "password" ? "new-password" : "off"}
          type={f.type || "text"}
          min={f.type === "number" ? 1 : undefined}
          max={f.type === "number" ? 65535 : undefined}
        />
      )}
    </label>
  );
}
