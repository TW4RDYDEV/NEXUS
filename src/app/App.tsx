import packageInfo from "../../package.json";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  lazy,
  Suspense,
} from "react";
import {
  Activity,
  ArrowRight,
  Camera,
  Check,
  ChevronDown,
  ChevronRight,
  Clock3,
  FileCheck,
  FolderOpen,
  GitBranch,
  HardDrive,
  LayoutDashboard,
  ListChecks,
  LockKeyhole,
  Network,
  PanelRightClose,
  PanelRightOpen,
  Plus,
  Search,
  Server,
  Settings2,
  Target,
  TerminalSquare,
  TriangleAlert,
  Upload,
  X,
} from "lucide-react";
import { api, chooseDirectory, chooseFile, type Row, time } from "../lib/api";
import { Brand, Modal, Loading, Badge } from "../components/ui";
import {
  WorkspaceContext,
  type Workspace,
  type View,
  type EditorSpec,
} from "../state/workspace";
import { Editor } from "../components/Editor";
import { Inspector } from "../components/Inspector";
import { Overview } from "../features/engagements/Overview";
import { EntityTable } from "../features/assets/EntityTable";
import { Credentials, VaultDialog } from "../features/credentials/Credentials";
import { Pivots } from "../features/pivots/Pivots";
import { Assessment } from "../features/assessment/Assessment";
import { Coverage } from "../features/coverage/Coverage";
import { Snapshots } from "../features/snapshots/Snapshots";
import { Scope, Settings } from "../features/engagements/Settings";
import { ImportDialog } from "../features/imports/ImportDialog";
import { Palette } from "../features/search/Palette";
import { isTauri } from "@tauri-apps/api/core";
const Graph = lazy(() =>
  import("../features/graph/Graph").then((m) => ({ default: m.Graph })),
);
const navigation: { name: View; icon: typeof Network; section?: string }[] = [
  { name: "Overview", icon: LayoutDashboard },
  { name: "Graph", icon: Network },
  { name: "Assets", icon: Server },
  { name: "Credentials", icon: LockKeyhole, section: "ACCESS" },
  { name: "Sessions", icon: TerminalSquare },
  { name: "Pivots", icon: GitBranch },
  { name: "Findings", icon: TriangleAlert, section: "ASSESSMENT" },
  { name: "Evidence", icon: FileCheck },
  { name: "Assessment", icon: ListChecks },
  { name: "Coverage", icon: ListChecks },
  { name: "Timeline", icon: Clock3, section: "HISTORY" },
  { name: "Snapshots", icon: Camera },
];
const emptyLookup = {
  assets: [],
  services: [],
  credentials: [],
  credential_tests: [],
  sessions: [],
  pivots: [],
  findings: [],
  evidence: [],
} as Record<string, Row[]>;
export function App() {
  const [welcome, setWelcome] = useState<Row | null>(null);
  const [summary, setSummary] = useState<Row | null>(null);
  const [lookup, setLookup] = useState(emptyLookup);
  const [view, setView] = useState<View>("Overview");
  const [history, setHistory] = useState<string[]>([]);
  const [inspectorVisible, setInspectorVisible] = useState(true);
  const [graphFocus, setGraphFocus] = useState("");
  const [changed, setChanged] = useState<Row[]>([]);
  const [revision, setRevision] = useState(0);
  const [editor, setEditor] = useState<EditorSpec | null>(null);
  const [importing, setImporting] = useState(false);
  const [vault, setVault] = useState(false);
  const [palette, setPalette] = useState(false);
  const [activity, setActivity] = useState(false);
  const [toast, setToast] = useState<{
    message: string;
    error: boolean;
  } | null>(null);
  const [saveState, setSaveState] = useState("Saved");
  const [starting, setStarting] = useState(false);
  const [create, setCreate] = useState(false);
  const [opening, setOpening] = useState(false);
  const [openMode, setOpenMode] = useState("open");
  const [openPath, setOpenPath] = useState("");
  const [splash, setSplash] = useState(true);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const [importInitial, setImportInitial] = useState<
    { name: string; text: string } | undefined
  >();
  const notify = useCallback((message: string, error = false) => {
    clearTimeout(toastTimer.current);
    setToast({ message, error });
    if (!error) toastTimer.current = setTimeout(() => setToast(null), 4500);
  }, []);
  const loadWelcome = useCallback(
    () =>
      api("welcome")
        .then(setWelcome)
        .catch((e) => notify(e.message, true)),
    [notify],
  );
  useEffect(() => {
    loadWelcome();
    const timer = setTimeout(() => setSplash(false), 400);
    return () => clearTimeout(timer);
  }, [loadWelcome]);
  const refresh = useCallback(async () => {
    const next = await api("summary");
    const entries = await Promise.all(
      Object.keys(emptyLookup).map(async (table) => [
        table,
        (
          await api("list", {
            table,
            limit: 100,
            sort: table === "credential_tests" ? "created_at" : "name",
            descending: table === "credential_tests",
          })
        ).rows,
      ]),
    );
    setSummary(next);
    setLookup(Object.fromEntries(entries));
    setRevision((r) => r + 1);
  }, []);
  const run = useCallback(
    async <T = any,>(
      op: string,
      args: Row = {},
      message?: string,
    ): Promise<T> => {
      setSaveState("Saving…");
      try {
        const result = await api<T>(op, args);
        await refresh();
        setSaveState("Saved");
        if (message) notify(message);
        return result;
      } catch (e) {
        setSaveState("Error saving");
        notify(e instanceof Error ? e.message : "Operation failed", true);
        throw e;
      }
    },
    [refresh, notify],
  );
  useEffect(() => {
    if (!summary) return;
    const timer = setInterval(
      () =>
        api("summary")
          .then(setSummary)
          .catch((e) => notify(e.message, true)),
      30000,
    );
    return () => clearInterval(timer);
  }, [Boolean(summary), notify]);
  useEffect(() => {
    if (!summary) return;
    document.documentElement.dataset.density =
      summary.settings.density || "compact";
    document.documentElement.dataset.motion =
      summary.settings.reduced_motion === "true" ? "reduced" : "normal";
  }, [summary?.settings]);
  const select = useCallback((id: string) => {
    setHistory((h) => (h.at(-1) === id ? h : [...h.slice(-24), id]));
    setInspectorVisible(true);
  }, []);
  const navigate = useCallback((next: View) => {
    setView(next);
    setPalette(false);
  }, []);
  const focus = useCallback((id: string) => {
    setGraphFocus(id);
    setView("Graph");
  }, []);
  const edit = useCallback(
    (table: string, data?: Row) => {
      if (table === "credentials" && !summary?.vault_unlocked) {
        setVault(true);
        return;
      }
      setEditor({ table, data });
      setPalette(false);
    },
    [summary?.vault_unlocked],
  );
  const close = useCallback(async () => {
    try {
      await api("close");
      setSummary(null);
      setHistory([]);
      setLookup(emptyLookup);
      setGraphFocus("");
      loadWelcome();
    } catch (e) {
      notify(
        e instanceof Error ? e.message : "Unable to close engagement",
        true,
      );
    }
  }, [loadWelcome, notify]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      if (e.key.toLowerCase() === "k" && summary) {
        e.preventDefault();
        setPalette((p) => !p);
      }
      if (
        e.key.toLowerCase() === "z" &&
        summary &&
        !["INPUT", "TEXTAREA"].includes((e.target as HTMLElement).tagName)
      ) {
        e.preventDefault();
        run("undo", {}, "Manual edit undone").catch(() => {});
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [summary, run]);
  useEffect(() => {
    if (!isTauri() || !summary) return;
    let off: (() => void) | undefined;
    import("@tauri-apps/api/webviewWindow").then(
      async ({ getCurrentWebviewWindow }) => {
        off = await getCurrentWebviewWindow().onDragDropEvent(async (event) => {
          if (event.payload.type === "drop" && event.payload.paths[0])
            try {
              const initial = await api("read_import", {
                path: event.payload.paths[0],
              });
              setImportInitial(initial);
              setImporting(true);
            } catch (e) {
              notify(
                e instanceof Error ? e.message : "Cannot read dropped file",
                true,
              );
            }
        });
      },
    );
    return () => off?.();
  }, [Boolean(summary), notify]);
  async function launch(op: string, args: Row = {}) {
    setStarting(true);
    try {
      await api(op, args);
      await refresh();
      setCreate(false);
      setOpening(false);
      setHistory([]);
      setView("Overview");
      setGraphFocus("");
      setChanged([]);
      setSaveState("Saved");
    } catch (e) {
      notify(
        e instanceof Error ? e.message : "Unable to open engagement",
        true,
      );
    } finally {
      setStarting(false);
    }
  }
  const importFile = () => {
    setImportInitial(undefined);
    setImporting(true);
    setPalette(false);
  };
  const selected = history.at(-1) || null;
  const context: Workspace = {
    summary: summary || {},
    lookup,
    view,
    revision,
    selected,
    graphFocus,
    changed,
    setChanged,
    navigate,
    select,
    focus,
    edit,
    refresh,
    run,
    notify,
    importFile,
    openVault: () => setVault(true),
    close,
  };
  let content;
  if (view === "Overview") content = <Overview />;
  else if (view === "Graph")
    content = (
      <Suspense fallback={<Loading label="Loading graph engine" />}>
        <Graph />
      </Suspense>
    );
  else if (view === "Credentials") content = <Credentials />;
  else if (view === "Pivots") content = <Pivots />;
  else if (view === "Assessment") content = <Assessment />;
  else if (view === "Coverage") content = <Coverage />;
  else if (view === "Snapshots") content = <Snapshots />;
  else if (view === "Scope") content = <Scope />;
  else if (view === "Settings") content = <Settings />;
  else
    content = (
      <EntityTable
        key={view}
        table={
          {
            Assets: "assets",
            Sessions: "sessions",
            Findings: "findings",
            Evidence: "evidence",
            Timeline: "timeline_events",
          }[view] || "assets"
        }
      />
    );
  return (
    <WorkspaceContext.Provider value={context}>
      {splash ? (
        <div className="splash">
          <Brand large />
          <p>Offensive Security Engagement Platform</p>
          <span className="splash-line" />
        </div>
      ) : !summary ? (
        <main className="welcome">
          <div className="welcome-top">
            <Brand />
            <span>OFFENSIVE SECURITY ENGAGEMENT PLATFORM</span>
            <span className="version">{packageInfo.version}</span>
          </div>
          <div className="welcome-content">
            <div className="welcome-eyebrow">
              <span className="status-dot" />
              LOCAL-FIRST. CONTEXT-DRIVEN.
            </div>
            <h1>
              Every observation.
              <br />
              One connected workspace.
            </h1>
            <p>Map assets. Track access. Understand the path.</p>
            <div className="welcome-actions">
              <button
                className="primary"
                disabled={starting}
                onClick={() => setCreate(true)}
              >
                <Plus size={16} />
                Create engagement
                <ArrowRight size={15} />
              </button>
              <button
                disabled={starting}
                onClick={async () => {
                  if (isTauri()) {
                    const path = await chooseDirectory();
                    if (path) launch("open", { path });
                  } else {
                    setOpenMode("open");
                    setOpening(true);
                  }
                }}
              >
                <FolderOpen size={16} />
                Open engagement
              </button>
              <button
                className="demo-button"
                disabled={starting}
                onClick={() => launch("demo")}
              >
                <Network size={16} />
                {starting ? "Opening workspace…" : "Explore demo"}
                <Badge tone="neutral">DEMO DATA</Badge>
              </button>
            </div>
            {welcome?.recent.length > 0 && (
              <div className="recent-workspaces">
                <h2>Recent engagements</h2>
                {welcome?.recent.slice(0, 5).map((r: Row) => (
                  <button
                    key={r.path}
                    disabled={starting}
                    onClick={() => launch("open", { path: r.path })}
                  >
                    <div className="workspace-avatar">{r.name.slice(0, 1)}</div>
                    <div>
                      <strong>{r.name}</strong>
                      <span>
                        {r.status}
                        {r.demo ? " · Demo data" : ""}
                      </span>
                    </div>
                    <ChevronRight size={16} />
                  </button>
                ))}
              </div>
            )}
            <button
              className="text-button"
              disabled={starting}
              onClick={async () => {
                if (isTauri()) {
                  const path = await chooseFile();
                  if (path) launch("recover", { path });
                } else {
                  setOpenMode("recover");
                  setOpening(true);
                }
              }}
            >
              Recover from a local backup
            </button>
            <div className="welcome-principles">
              <span>
                <HardDrive size={15} />
                Your data stays local
              </span>
              <span>
                <LockKeyhole size={15} />
                Encrypted credential vault
              </span>
              <span>
                <Network size={15} />
                Evidence-backed relationships
              </span>
            </div>
          </div>
          <footer className="welcome-footer">
            NEXUS <span>by TWARDY.exe</span>
            <span>Built for the work between the tools.</span>
          </footer>
        </main>
      ) : (
        <div
          className={`app-shell ${inspectorVisible && selected ? "has-inspector" : ""}`}
        >
          <aside className="sidebar">
            <div className="sidebar-brand">
              <Brand />
              <span className="edition">WORKSPACE</span>
            </div>
            <button className="engagement-switcher" onClick={close}>
              <div className="workspace-avatar">
                {summary.engagement.name.slice(0, 1)}
              </div>
              <span>
                <strong>{summary.engagement.name}</strong>
                <small>
                  {summary.engagement.demo
                    ? "DEMO DATA"
                    : summary.engagement.kind}
                </small>
              </span>
              <ChevronDown size={14} />
            </button>
            <button className="sidebar-search" onClick={() => setPalette(true)}>
              <Search size={14} />
              <span>Search anything</span>
              <kbd>⌘ K</kbd>
            </button>
            <nav aria-label="Main navigation">
              {navigation.map(({ name, icon: Icon, section }) => (
                <div key={name}>
                  {section && <div className="nav-section">{section}</div>}
                  <button
                    className={view === name ? "active" : ""}
                    onClick={() => navigate(name)}
                  >
                    <Icon size={16} />
                    <span>{name}</span>
                    {["Assets", "Credentials", "Findings"].includes(name) && (
                      <small>{summary.counts[name.toLowerCase()]}</small>
                    )}
                  </button>
                </div>
              ))}
            </nav>
            <div className="sidebar-bottom">
              <button
                className={view === "Scope" ? "active" : ""}
                onClick={() => navigate("Scope")}
              >
                <Target size={16} />
                Scope Guard<small>{summary.scope.length}</small>
              </button>
              <button
                className={view === "Settings" ? "active" : ""}
                onClick={() => navigate("Settings")}
              >
                <Settings2 size={16} />
                Settings
              </button>
              <div className="sidebar-watermark">
                <strong>NEXUS</strong>
                <span>by TWARDY.exe</span>
                <i>{packageInfo.version}</i>
              </div>
            </div>
          </aside>
          <div className="workspace-shell">
            <header className="topbar">
              <div className="breadcrumb">
                <span>{summary.engagement.name}</span>
                <ChevronRight size={12} />
                <strong>{view}</strong>
              </div>
              <div className="topbar-actions">
                {summary.engagement.demo === 1 && (
                  <span className="demo-label">DEMO DATA</span>
                )}
                <span
                  className={`save-status ${saveState === "Error saving" ? "bad" : ""}`}
                >
                  {saveState === "Saved" ? (
                    <Check size={12} />
                  ) : saveState === "Saving…" ? (
                    <RefreshIcon />
                  ) : (
                    <TriangleAlert size={12} />
                  )}{" "}
                  {saveState}
                </span>
                <button
                  title="Import observations"
                  className="icon-button"
                  aria-label="Import observations"
                  onClick={importFile}
                >
                  <Upload size={15} />
                </button>
                <button
                  title="Activity drawer"
                  className={`icon-button ${activity ? "active" : ""}`}
                  aria-label="Toggle activity drawer"
                  onClick={() => setActivity(!activity)}
                >
                  <Activity size={16} />
                </button>
                <button
                  title="Toggle inspector"
                  className="icon-button"
                  aria-label="Toggle inspector"
                  onClick={() => setInspectorVisible(!inspectorVisible)}
                >
                  {inspectorVisible ? (
                    <PanelRightClose size={16} />
                  ) : (
                    <PanelRightOpen size={16} />
                  )}
                </button>
              </div>
            </header>
            <div className="workspace-body">
              <main className="main-content">{content}</main>
              {inspectorVisible && selected && (
                <Inspector
                  onClose={() => setInspectorVisible(false)}
                  onBack={() => setHistory((h) => h.slice(0, -1))}
                  canBack={history.length > 1}
                />
              )}
            </div>
            {activity && (
              <section className="activity-drawer">
                <header>
                  <span>
                    <Activity size={13} />
                    ACTIVITY
                  </span>
                  <button
                    className="icon-button"
                    aria-label="Close activity drawer"
                    onClick={() => setActivity(false)}
                  >
                    <X size={14} />
                  </button>
                </header>
                <div>
                  {summary.timeline.slice(0, 30).map((e: Row) => (
                    <button
                      key={e.id}
                      onClick={() =>
                        e.entity_id ? select(e.entity_id) : navigate("Timeline")
                      }
                    >
                      <time>{time(e.created_at)}</time>
                      <span>{e.name}</span>
                      <small>{e.source}</small>
                    </button>
                  ))}
                </div>
              </section>
            )}
            <footer className="statusbar">
              <span>
                <span className="status-dot" />
                Local SQLite workspace
              </span>
              <button
                onClick={() =>
                  summary.vault_unlocked
                    ? run("vault_lock", {}, "Vault locked").catch(() => {})
                    : setVault(true)
                }
              >
                <LockKeyhole size={11} />
                Vault {summary.vault_unlocked ? "unlocked" : "locked"}
              </button>
              <span>
                {summary.counts.assets} assets · {summary.counts.services}{" "}
                services
              </span>
              <button onClick={() => navigate("Scope")}>
                <Target size={11} />
                Scope Guard enabled
              </button>
            </footer>
          </div>
        </div>
      )}
      {create && (
        <CreateEngagement
          defaultDirectory={welcome?.default_workspace}
          busy={starting}
          onClose={() => setCreate(false)}
          onCreate={(args) => launch("create", args)}
        />
      )}{" "}
      {opening && (
        <Modal
          title={
            openMode === "recover" ? "Recover a backup" : "Open engagement"
          }
          subtitle="Choose a directory containing a .nexus workspace, or the .nexus directory itself."
          onClose={() => setOpening(false)}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              launch(openMode, { path: openPath });
            }}
          >
            <label>
              Workspace path
              <input
                value={openPath}
                onChange={(e) => setOpenPath(e.target.value)}
                required
                autoFocus
              />
            </label>
            <footer>
              <button type="button" onClick={() => setOpening(false)}>
                Cancel
              </button>
              <button className="primary" disabled={starting}>
                Open engagement
              </button>
            </footer>
          </form>
        </Modal>
      )}
      {summary && editor && (
        <Editor spec={editor} onClose={() => setEditor(null)} />
      )}{" "}
      {summary && importing && (
        <ImportDialog
          initial={importInitial}
          onClose={() => {
            setImporting(false);
            setImportInitial(undefined);
          }}
        />
      )}
      {summary && vault && <VaultDialog onClose={() => setVault(false)} />}{" "}
      {summary && palette && <Palette onClose={() => setPalette(false)} />}{" "}
      {toast && (
        <div
          role={toast.error ? "alert" : "status"}
          className={`toast ${toast.error ? "error" : ""}`}
        >
          {toast.error ? <TriangleAlert size={16} /> : <Check size={16} />}
          <span>{toast.message}</span>
          <button
            className="icon-button"
            aria-label="Dismiss notification"
            onClick={() => setToast(null)}
          >
            <X size={14} />
          </button>
        </div>
      )}
    </WorkspaceContext.Provider>
  );
}
function RefreshIcon() {
  return <span className="tiny-spinner" />;
}
function CreateEngagement({
  onClose,
  onCreate,
  busy,
  defaultDirectory,
}: {
  onClose: () => void;
  onCreate: (v: Row) => void;
  busy: boolean;
  defaultDirectory: string;
}) {
  const [data, setData] = useState<Row>({
    name: "",
    kind: "Internal Pentest",
    description: "",
    client: "",
    start_date: new Date().toISOString().slice(0, 10),
    directory: defaultDirectory || "",
  });
  return (
    <Modal
      title="Create engagement"
      subtitle="A separate, portable workspace for every assessment."
      onClose={onClose}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onCreate(data);
        }}
      >
        <div className="form-grid">
          <label className="wide-field">
            Engagement name
            <input
              required
              autoFocus
              value={data.name}
              onChange={(e) => setData({ ...data, name: e.target.value })}
            />
          </label>
          <label>
            Engagement type
            <select
              value={data.kind}
              onChange={(e) => setData({ ...data, kind: e.target.value })}
            >
              {[
                "Lab / CTF",
                "Internal Pentest",
                "External Pentest",
                "Web / API",
                "Other",
              ].map((t) => (
                <option key={t}>{t}</option>
              ))}
            </select>
          </label>
          <label>
            Start date
            <input
              type="date"
              value={data.start_date}
              onChange={(e) => setData({ ...data, start_date: e.target.value })}
            />
          </label>
          <label className="wide-field">
            Description
            <textarea
              rows={3}
              value={data.description}
              onChange={(e) =>
                setData({ ...data, description: e.target.value })
              }
            />
          </label>
          <label className="wide-field">
            Client / project reference
            <input
              value={data.client}
              onChange={(e) => setData({ ...data, client: e.target.value })}
            />
          </label>
          <label className="wide-field">
            Workspace parent directory
            <div className="input-with-button">
              <input
                value={data.directory}
                onChange={(e) =>
                  setData({ ...data, directory: e.target.value })
                }
              />
              {isTauri() && (
                <button
                  type="button"
                  aria-label="Browse workspace directory"
                  onClick={async () => {
                    const directory = await chooseDirectory();
                    if (directory) setData({ ...data, directory });
                  }}
                >
                  <FolderOpen size={15} />
                </button>
              )}
            </div>
          </label>
        </div>
        <footer>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button className="primary" disabled={busy}>
            {busy ? "Creating…" : "Create engagement"}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
