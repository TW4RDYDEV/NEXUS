import { useEffect, useState } from "react";
import { Search, ChevronRight } from "lucide-react";
import { Modal, Badge } from "../../components/ui";
import { api, type Row } from "../../lib/api";
import { useWorkspace, type View } from "../../state/workspace";
export function Palette({ onClose }: { onClose: () => void }) {
  const w = useWorkspace();
  const [q, setQ] = useState("");
  const [results, setResults] = useState<Row[]>([]);
  const [selected, setSelected] = useState(0);
  const commands = [
    { name: "Create finding", action: () => w.edit("findings") },
    { name: "Add credential", action: () => w.edit("credentials") },
    { name: "Add session", action: () => w.edit("sessions") },
    { name: "Import file", action: w.importFile },
    { name: "Switch engagement", action: w.close },
    ...(
      [
        "Graph",
        "Assets",
        "Credentials",
        "Pivots",
        "Snapshots",
        "Coverage",
        "Scope",
        "Settings",
      ] as View[]
    ).map((v) => ({
      name:
        v === "Snapshots"
          ? "Create or compare snapshots"
          : `Open ${v.toLowerCase()}`,
      action: () => w.navigate(v),
    })),
  ];
  const filtered = commands.filter((c) =>
    c.name.toLowerCase().includes(q.toLowerCase()),
  );
  const actions = [
    ...results.map((r) => ({
      name: r.name,
      detail: r.detail,
      kind: r.table,
      action: () => w.select(r.id),
    })),
    ...filtered.map((c) => ({ ...c, detail: "Command", kind: "command" })),
  ];
  useEffect(() => {
    let current = true;
    setSelected(0);
    if (!q.trim()) {
      setResults([]);
      return;
    }
    const timer = setTimeout(
      () =>
        api("search", { q })
          .then((r) => {
            if (current) setResults(r);
          })
          .catch((e) => w.notify(e.message, true)),
      150,
    );
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [q]);
  const execute = (index: number) => {
    const action = actions[index];
    if (action) {
      action.action();
      onClose();
    }
  };
  return (
    <Modal title="Search & commands" onClose={onClose}>
      <div className="palette-input">
        <Search size={19} />
        <input
          autoFocus
          aria-label="Search everything"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelected((i) => Math.min(actions.length - 1, i + 1));
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelected((i) => Math.max(0, i - 1));
            }
            if (e.key === "Enter") {
              e.preventDefault();
              execute(selected);
            }
          }}
        />
        <kbd>ESC</kbd>
      </div>
      <div
        className="palette-results"
        role="listbox"
        aria-label="Search results"
      >
        {actions.length ? (
          actions.map((item, index) => (
            <button
              key={item.kind + item.name + index}
              role="option"
              aria-selected={selected === index}
              className={selected === index ? "active" : ""}
              onMouseEnter={() => setSelected(index)}
              onClick={() => execute(index)}
            >
              <span>
                <strong>{item.name}</strong>
                <small>{item.detail}</small>
              </span>
              <Badge>{item.kind.replaceAll("_", " ")}</Badge>
              <ChevronRight size={14} />
            </button>
          ))
        ) : (
          <p className="secondary section-padding">
            No matching entities or commands.
          </p>
        )}
      </div>
      <div className="palette-hint">
        ↑ ↓ to navigate <span>Enter to open</span>
        <span>Secrets are excluded from search</span>
      </div>
    </Modal>
  );
}
