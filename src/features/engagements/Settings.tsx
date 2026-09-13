import { Delivery } from "./Delivery";
import { useEffect, useState } from "react";
import {
  Plus,
  Trash2,
  HardDrive,
  LockKeyhole,
  RefreshCw,
  Play,
  Square,
  FolderOpen,
} from "lucide-react";
import { PageTitle, Badge, Modal, Brand } from "../../components/ui";
import { api, type Row, time, download } from "../../lib/api";
import { useWorkspace } from "../../state/workspace";
export function Scope() {
  const w = useWorkspace();
  const [rule, setRule] = useState("");
  const [excluded, setExcluded] = useState(false);
  return (
    <div className="page">
      <PageTitle
        eyebrow="ENGAGEMENT BOUNDARIES"
        title="Scope Guard"
        description="Exclusions always win. The integrated runner validates the resolved IPv4 target before execution."
      />
      <div className="scope-layout">
        {[false, true].map((ex) => (
          <section className="scope-section" key={String(ex)}>
            <div className="section-label">
              <h2>{ex ? "Explicit exclusions" : "Included targets"}</h2>
              <Badge tone={ex ? "bad" : "good"}>
                {
                  w.summary.scope.filter((r: Row) => Boolean(r.excluded) === ex)
                    .length
                }{" "}
                rules
              </Badge>
            </div>
            {w.summary.scope
              .filter((r: Row) => Boolean(r.excluded) === ex)
              .map((r: Row) => (
                <div className="scope-rule" key={r.id}>
                  <span className="mono">{r.rule}</span>
                  <button
                    aria-label={`Remove scope rule ${r.rule}`}
                    title="Remove rule"
                    className="icon-button"
                    onClick={() =>
                      w
                        .run("remove_scope", { id: r.id }, "Scope rule removed")
                        .catch(() => {})
                    }
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
            {!w.summary.scope.some((r: Row) => Boolean(r.excluded) === ex) && (
              <p className="secondary section-padding">
                {ex
                  ? "No explicit exclusions."
                  : "Add an inclusion rule to permit active scans."}
              </p>
            )}
          </section>
        ))}
      </div>
      <form
        className="scope-add"
        onSubmit={(e) => {
          e.preventDefault();
          w.run(
            "write",
            {
              table: "scope_rules",
              data: { rule: rule.trim(), excluded: excluded ? 1 : 0 },
            },
            "Scope rule added",
          )
            .then(() => setRule(""))
            .catch(() => {});
        }}
      >
        <label>
          IPv4, CIDR, hostname, or wildcard suffix
          <input
            value={rule}
            required
            onChange={(e) => setRule(e.target.value)}
            aria-label="Scope rule"
          />
        </label>
        <label>
          Rule type
          <select
            value={excluded ? "exclude" : "include"}
            onChange={(e) => setExcluded(e.target.value === "exclude")}
          >
            <option value="include">Include</option>
            <option value="exclude">Exclude</option>
          </select>
        </label>
        <button className="primary">
          <Plus size={14} />
          Add rule
        </button>
      </form>
      <div className="scope-explanation">
        <h3>How scope matching works</h3>
        <p>
          IPv4 networks use CIDR containment. A wildcard such as{" "}
          <code>*.corp.test</code> matches subdomains at a label boundary.
          Exclusions override every inclusion. Imports are passive observations
          and may retain out-of-scope assets.
        </p>
        <p>
          The Nmap runner accepts one host and pins execution to one resolved
          IPv4 address. That address must have an inclusion rule. It never
          accepts arbitrary shell arguments.
        </p>
      </div>
    </div>
  );
}
export function Settings() {
  const w = useWorkspace();
  const [integrations, setIntegrations] = useState<Row[]>([]);
  const [backups, setBackups] = useState<Row[]>([]);
  const [restore, setRestore] = useState<Row | null>(null);
  const [runner, setRunner] = useState(false);
  const [build, setBuild] = useState<Row | null>(null);
  const [tab, setTab] = useState("General");
  useEffect(() => {
    api("integrations")
      .then(setIntegrations)
      .catch((e) => w.notify(e.message, true));
    api("list", {
      table: "backups",
      limit: 20,
      sort: "created_at",
      descending: true,
    })
      .then((r) => setBackups(r.rows))
      .catch((e) => w.notify(e.message, true));
  }, [w.revision]);
  useEffect(() => {
    api("build_metadata")
      .then(setBuild)
      .catch((e) => w.notify(e.message, true));
  }, []);
  const update = (key: string, value: string) =>
    w.run("settings", { [key]: value }, "Preference saved").catch(() => {});
  return (
    <div className="page settings">
      <PageTitle
        eyebrow="WORKSPACE PREFERENCES"
        title="Settings"
        description="Local controls for storage, security, and integrated tools."
      />
      <div className="settings-tabs segmented">
        {["General", "Security", "Integrations", "About"].map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={tab === t ? "active" : ""}
          >
            {t}
          </button>
        ))}
      </div>
      {tab === "General" && (
        <div className="settings-section">
          <h2>Appearance & workspace</h2>
          <div className="setting-row">
            <div>
              <strong>Table density</strong>
              <p>Spacing for long assessment sessions.</p>
            </div>
            <select
              aria-label="Table density"
              value={w.summary.settings.density || "compact"}
              onChange={(e) => update("density", e.target.value)}
            >
              <option value="compact">Compact</option>
              <option value="comfortable">Comfortable</option>
            </select>
          </div>
          <div className="setting-row">
            <div>
              <strong>Reduced motion</strong>
              <p>Disable interface transitions.</p>
            </div>
            <input
              aria-label="Reduced motion"
              type="checkbox"
              checked={w.summary.settings.reduced_motion === "true"}
              onChange={(e) =>
                update("reduced_motion", String(e.target.checked))
              }
            />
          </div>
          <div className="setting-row">
            <div>
              <strong>Engagement workspace</strong>
              <p className="mono path-text">{w.summary.workspace}</p>
              <p>
                Each engagement contains its database, imports, evidence,
                exports, and backups.
              </p>
            </div>
            <HardDrive size={20} />
          </div>
          <button onClick={w.close}>
            <FolderOpen size={14} />
            Switch engagement
          </button>
        </div>
      )}
      {tab === "Security" && (
        <div className="settings-section">
          <Delivery />
          <h2>Vault & recovery</h2>
          <div className="setting-row">
            <div>
              <strong>Credential vault</strong>
              <p>
                Passphrase-derived encryption key exists only in memory while
                unlocked.
              </p>
            </div>
            <button
              onClick={() =>
                w.summary.vault_unlocked
                  ? w.run("vault_lock", {}, "Vault locked").catch(() => {})
                  : w.openVault()
              }
            >
              <LockKeyhole size={14} />
              {w.summary.vault_unlocked ? "Lock vault" : "Unlock vault"}
            </button>
          </div>
          <div className="setting-row">
            <div>
              <strong>Vault inactivity timeout</strong>
              <p>Secrets re-lock when no vault operation occurs.</p>
            </div>
            <select
              aria-label="Vault inactivity timeout"
              value={w.summary.settings.auto_lock || "300"}
              onChange={(e) => update("auto_lock", e.target.value)}
            >
              {[
                ["60", "1 minute"],
                ["300", "5 minutes"],
                ["900", "15 minutes"],
                ["1800", "30 minutes"],
              ].map(([v, l]) => (
                <option key={v} value={v}>
                  {l}
                </option>
              ))}
            </select>
          </div>
          <div className="setting-row">
            <div>
              <strong>Automatic local backups</strong>
              <p>
                Checked while the workspace is open. The newest 20 database
                backups are retained.
              </p>
            </div>
            <select
              aria-label="Backup interval"
              value={w.summary.settings.backup_minutes || "10"}
              onChange={(e) => update("backup_minutes", e.target.value)}
            >
              {["5", "10", "15", "30", "60"].map((v) => (
                <option key={v} value={v}>
                  Every {v} minutes
                </option>
              ))}
            </select>
          </div>
          <div className="section-label">
            <h3>Available backups</h3>
            <button
              onClick={() =>
                w.run("backup", {}, "Backup created").catch(() => {})
              }
            >
              <Plus size={14} />
              Back up now
            </button>
          </div>
          {backups.map((b) => (
            <div className="backup-row" key={b.id}>
              <HardDrive size={15} />
              <div>
                <strong>{time(b.created_at)}</strong>
                <span className="mono">{b.name}</span>
              </div>
              <button onClick={() => setRestore(b)}>Restore copy</button>
            </div>
          ))}
        </div>
      )}
      {tab === "Integrations" && (
        <div className="settings-section">
          <div className="section-label">
            <h2>Local tool detection</h2>
            <button
              onClick={() =>
                api("integrations")
                  .then(setIntegrations)
                  .catch((e) => w.notify(e.message, true))
              }
            >
              <RefreshCw size={14} />
              Refresh
            </button>
          </div>
          <p className="secondary">
            NEXUS does not bundle or download assessment tools. All four
            adapters can import observations independently of local
            installation.
          </p>
          {integrations.map((i) => (
            <div className="integration-row" key={i.name}>
              <div className="tool-monogram">{i.name.slice(0, 1)}</div>
              <div>
                <strong>{i.name}</strong>
                <p className="mono">
                  {i.path || "Executable not found in PATH"}
                </p>
              </div>
              <Badge>{i.installed ? "Installed" : "Missing"}</Badge>
              {i.runner && (
                <button
                  disabled={!i.installed}
                  title={
                    !i.installed
                      ? "Install Nmap and add it to PATH to enable the runner"
                      : "Prepare an explicitly scoped Nmap scan"
                  }
                  onClick={() => setRunner(true)}
                >
                  <Play size={14} />
                  Prepare scan
                </button>
              )}
            </div>
          ))}
        </div>
      )}
      {tab === "About" && (
        <div className="about">
          <Brand large />
          <p className="version">Version {build?.application_version || "…"}</p>
          <p className="secondary">NEXUS — TWARDY.exe / TW4RDYDEV</p>
          <h2>Map assets. Track access. Understand the path.</h2>
          <p>
            NEXUS is a local-first offensive-security engagement workspace that
            correlates assets, credentials, access, pivots, findings, and
            evidence into one operational graph.
          </p>
          {build && (
            <>
              <dl className="build-metadata">
                <dt>Product ID</dt>
                <dd>{build.product_id}</dd>
                <dt>Schema</dt>
                <dd>
                  {build.schema_version} · {build.schema_family}
                </dd>
                <dt>Build</dt>
                <dd>
                  {build.build_mode} · commit/{build.git_commit}
                </dd>
              </dl>
              <button
                onClick={() =>
                  api("diagnostic")
                    .then((metadata) =>
                      download(
                        "NEXUS-build-diagnostic.json",
                        JSON.stringify(metadata, null, 2) + "\n",
                      ),
                    )
                    .catch((e) => w.notify(e.message, true))
                }
              >
                Export local build diagnostic
              </button>
              <p className="secondary">
                Contains product and build metadata only. Saved locally when
                requested.
              </p>
            </>
          )}
          <div className="about-facts">
            <span>React + TypeScript</span>
            <span>Rust + SQLite</span>
            <span>Tauri 2</span>
          </div>
          <p className="secondary">
            Built for authorized penetration testing, research, and lab
            assessments. No telemetry, analytics, cloud synchronization, or LLM
            dependency.
          </p>
          <p className="secondary">
            Source-available under the NEXUS Source-Available License 1.0.
            Private modifications are permitted; redistribution and rebranding
            require authorization.
          </p>
        </div>
      )}
      {restore && (
        <Modal
          title="Restore backup into a separate workspace?"
          subtitle="The original engagement remains intact. Attachments and retained imports are copied into the recovered workspace."
          onClose={() => setRestore(null)}
        >
          <p>{time(restore.created_at)}</p>
          <footer>
            <button onClick={() => setRestore(null)}>Cancel</button>
            <button
              className="primary"
              onClick={() =>
                w
                  .run(
                    "restore",
                    { name: restore.name, confirmed: true },
                    "Recovered workspace opened",
                  )
                  .then(() => setRestore(null))
                  .catch(() => {})
              }
            >
              Restore copy
            </button>
          </footer>
        </Modal>
      )}
      {runner && <Runner onClose={() => setRunner(false)} />}
    </div>
  );
}
function Runner({ onClose }: { onClose: () => void }) {
  const w = useWorkspace();
  const [target, setTarget] = useState("");
  const [ports, setPorts] = useState("22,80,443,445,3389,5985");
  const [plan, setPlan] = useState<Row | null>(null);
  const [job, setJob] = useState<Row | null>(null);
  const [error, setError] = useState("");
  const [running, setRunning] = useState(false);
  useEffect(() => {
    api("runner_status")
      .then((result) => {
        if (result) {
          setJob(result);
          setRunning(!result.done);
        }
      })
      .catch((e) => setError(e.message));
  }, []);
  useEffect(() => {
    if (!running) return;
    const timer = setInterval(
      () =>
        api("runner_status")
          .then((r) => {
            setJob(r);
            if (r?.done) {
              setRunning(false);
              w.refresh();
            }
          })
          .catch((e) => setError(e.message)),
      1000,
    );
    return () => clearInterval(timer);
  }, [running]);
  return (
    <Modal
      title="Scoped Nmap execution"
      subtitle="Reconnaissance with a fixed argument profile. Output is reviewed before import."
      onClose={onClose}
      wide
    >
      <div className="form-grid">
        <label>
          Target hostname or IPv4
          <input
            value={target}
            onChange={(e) => {
              setTarget(e.target.value);
              setPlan(null);
            }}
            disabled={running}
          />
        </label>
        <label>
          Ports · comma separated
          <input
            value={ports}
            onChange={(e) => {
              setPorts(e.target.value);
              setPlan(null);
            }}
            disabled={running}
          />
        </label>
      </div>
      {plan && (
        <div className="runner-plan">
          <Badge>Scope Guard passed</Badge>
          <p>
            Resolved target: <code>{plan.resolved}</code>
          </p>
          <pre>
            {plan.binary}
            {"\n"}
            {plan.args.join(" ")}
          </pre>
        </div>
      )}
      {error && (
        <div role="alert" className="error-box">
          {error}
        </div>
      )}
      {job && (
        <div className="runner-output">
          <div className="section-label">
            <h3>Captured output</h3>
            <Badge>
              {job.done ? (job.success ? "Completed" : "Stopped") : "Running"}
            </Badge>
          </div>
          <pre>{job.stderr || job.stdout || "Waiting for Nmap output…"}</pre>
          {job.done && job.success && (
            <button
              onClick={() => {
                onClose();
                w.importFile();
                w.notify(
                  `Scan output retained at ${job.path}. Choose it in the import dialog.`,
                );
              }}
            >
              Review scan import
            </button>
          )}
        </div>
      )}
      <footer>
        <button onClick={onClose}>
          {running ? "Continue in background" : "Close"}
        </button>
        {running ? (
          <button
            className="danger"
            onClick={() =>
              api("runner_status", { cancel: true })
                .then(setJob)
                .catch((e) => setError(e.message))
            }
          >
            <Square size={14} />
            Stop scan
          </button>
        ) : plan ? (
          <button
            className="primary"
            onClick={() => {
              setError("");
              w.run("runner_start", { target, ports }, "Nmap started")
                .then(() => setRunning(true))
                .catch((e) => setError(e.message));
            }}
          >
            <Play size={14} />
            Run displayed command
          </button>
        ) : (
          <button
            className="primary"
            onClick={() => {
              setError("");
              api("runner_plan", { target, ports })
                .then(setPlan)
                .catch((e) => {
                  setError(e.message);
                  w.refresh();
                });
            }}
          >
            Validate scope & preview
          </button>
        )}
      </footer>
    </Modal>
  );
}
