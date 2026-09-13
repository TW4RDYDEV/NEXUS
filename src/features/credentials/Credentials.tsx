import { useEffect, useState, useRef } from "react";
import {
  Plus,
  LockKeyhole,
  Unlock,
  Eye,
  Copy,
  Check,
  X,
  Minus,
} from "lucide-react";
import { api, copy, time, type Row } from "../../lib/api";
import { useWorkspace } from "../../state/workspace";
import { PageTitle, Badge, Empty, Modal } from "../../components/ui";
export function Credentials() {
  const w = useWorkspace();
  const [credentials, setCredentials] = useState<Row[]>([]);
  const [identityQuery, setIdentityQuery] = useState("");
  const [identityPage, setIdentityPage] = useState(0);
  const [identityTotal, setIdentityTotal] = useState(0);
  const [hostPage, setHostPage] = useState(0);
  const [historyPage, setHistoryPage] = useState(0);
  const [matrix, setMatrix] = useState<Row>({
    hosts: [],
    history: [],
    total: 0,
    history_total: 0,
  });
  const [error, setError] = useState("");
  const revealGeneration = useRef(0);
  const [matrixIdentity, setMatrixIdentity] = useState("");
  const [selected, setSelected] = useState("");
  const [mode, setMode] = useState("Access matrix");
  const [reveal, setReveal] = useState<{ id: string; secret: string } | null>(
    null,
  );
  const [q, setQ] = useState("");
  useEffect(() => {
    if (!credentials.some((c) => c.id === selected))
      setSelected(credentials[0]?.id || "");
  }, [credentials, selected]);
  useEffect(() => {
    if (!reveal) return;
    const timer = setTimeout(() => setReveal(null), 15000);
    const mask = () => {
      revealGeneration.current++;
      setReveal(null);
    };
    window.addEventListener("blur", mask);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("blur", mask);
    };
  }, [reveal]);
  useEffect(() => {
    if (!w.summary.vault_unlocked) {
      revealGeneration.current++;
      setReveal(null);
    }
  }, [w.summary.vault_unlocked]);
  useEffect(() => {
    let live = true;
    const timer = setTimeout(
      () =>
        api("list", {
          table: "credentials",
          q: identityQuery,
          limit: 50,
          offset: identityPage * 50,
          sort: "name",
        })
          .then((r) => {
            if (live) {
              setCredentials(r.rows);
              setIdentityTotal(r.total);
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
  }, [identityQuery, identityPage, w.revision]);
  useEffect(() => {
    let live = true;
    revealGeneration.current++;
    setReveal(null);
    if (!selected) return;
    const timer = setTimeout(
      () =>
        api("matrix_page", {
          credential_id: selected,
          q,
          offset: hostPage * 25,
          history_offset: historyPage * 50,
        })
          .then((r) => {
            if (live) {
              setMatrixIdentity(selected);
              setMatrix(r);
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
  }, [selected, q, hostPage, historyPage, w.revision]);
  const tests: Row[] = matrixIdentity === selected ? matrix.history : [];
  const protocols = [
    { name: "SMB", ports: [445, 139] },
    { name: "WinRM", ports: [5985, 5986] },
    { name: "SSH", ports: [22] },
    { name: "RDP", ports: [3389] },
    { name: "Other", ports: [] },
  ];
  const hosts: Row[] = matrixIdentity === selected ? matrix.hosts : [];
  const add = () =>
    w.summary.vault_unlocked ? w.edit("credentials") : w.openVault();
  return (
    <div className="page credentials-page">
      <PageTitle
        eyebrow="IDENTITY & ACCESS"
        title="Credential vault"
        description="Know which identities work, where they came from, and what remains untested."
        actions={
          <>
            <button
              onClick={() =>
                w.summary.vault_unlocked
                  ? w.run("vault_lock", {}, "Vault locked").catch(() => {})
                  : w.openVault()
              }
            >
              {w.summary.vault_unlocked ? (
                <Unlock size={14} />
              ) : (
                <LockKeyhole size={14} />
              )}
              Vault {w.summary.vault_unlocked ? "unlocked" : "locked"}
            </button>
            <button className="primary" onClick={add}>
              <Plus size={14} />
              Add credential
            </button>
          </>
        }
      />
      <div className="vault-banner">
        <LockKeyhole size={15} />
        <span>Secrets are encrypted on disk. Reveal lasts 15 seconds.</span>
        <Badge tone={w.summary.vault_unlocked ? "good" : "neutral"}>
          {w.summary.vault_unlocked ? "UNLOCKED" : "LOCKED"}
        </Badge>
      </div>
      {error && (
        <div role="alert" className="error-box">
          {error}
        </div>
      )}
      {w.summary.counts.credentials === 0 ? (
        <Empty
          title="No credentials recorded"
          detail="Initialize the encrypted vault, then record a discovered identity."
          action={
            <button className="primary" onClick={add}>
              Set up credential vault
            </button>
          }
        />
      ) : (
        <>
          <div className="credential-layout">
            <div className="credential-list">
              <div className="section-label">
                <h3>
                  Known identities <span>{identityTotal}</span>
                </h3>
              </div>
              <input
                className="identity-search"
                aria-label="Search identities"
                placeholder="Search all identities…"
                maxLength={200}
                value={identityQuery}
                onChange={(e) => {
                  setIdentityQuery(e.target.value);
                  setIdentityPage(0);
                }}
              />
              {credentials.length === 0 && (
                <p className="secondary">No matching identities.</p>
              )}
              {credentials.map((c) => (
                <button
                  key={c.id}
                  className={`credential-choice ${selected === c.id ? "active" : ""}`}
                  onClick={() => {
                    setSelected(c.id);
                    setHistoryPage(0);
                    setReveal(null);
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    w.select(c.id);
                  }}
                >
                  <div className="credential-mark">
                    {c.kind === "NTLM hash" ? "#" : "@"}
                  </div>
                  <div>
                    <strong className="mono">{c.username}</strong>
                    <span>
                      {c.context || "No domain"} · {c.kind}
                    </span>
                  </div>
                </button>
              ))}
              <div className="paged-controls">
                <button
                  aria-label="Previous identities"
                  disabled={!identityPage}
                  onClick={() => setIdentityPage(identityPage - 1)}
                >
                  ←
                </button>
                <span>
                  {identityPage + 1} /{" "}
                  {Math.max(1, Math.ceil(identityTotal / 50))}
                </span>
                <button
                  aria-label="Next identities"
                  disabled={(identityPage + 1) * 50 >= identityTotal}
                  onClick={() => setIdentityPage(identityPage + 1)}
                >
                  →
                </button>
              </div>
            </div>
            <div className="matrix-main">
              <div className="matrix-header">
                <div>
                  <span className="eyebrow">SELECTED IDENTITY</span>
                  <h2 className="mono">
                    {credentials.find((c) => c.id === selected)?.context}
                    {credentials.find((c) => c.id === selected)?.context
                      ? "\\"
                      : ""}
                    {credentials.find((c) => c.id === selected)?.username}
                  </h2>
                  <button
                    className="text-button"
                    onClick={() => w.select(selected)}
                  >
                    View source and details
                  </button>
                </div>
                <div className="actions">
                  <button
                    aria-label="Copy username"
                    title="Copy username"
                    onClick={() =>
                      copy(
                        credentials.find((c) => c.id === selected)?.username ||
                          "",
                      )
                        .then(() => w.notify("Username copied"))
                        .catch((e) => w.notify(e.message, true))
                    }
                  >
                    <Copy size={14} />
                  </button>
                  <button
                    onClick={() => {
                      if (!w.summary.vault_unlocked) {
                        w.openVault();
                        return;
                      }
                      const generation = ++revealGeneration.current;
                      api("reveal", { id: selected })
                        .then(
                          (r) =>
                            generation === revealGeneration.current &&
                            document.hasFocus() &&
                            setReveal({ id: selected, secret: r.secret }),
                        )
                        .catch((e) => w.notify(e.message, true));
                    }}
                  >
                    <Eye size={14} />
                    Reveal
                  </button>
                </div>
              </div>
              <div className="table-toolbar">
                <div className="segmented">
                  {["Access matrix", "Test history"].map((m) => (
                    <button
                      key={m}
                      className={mode === m ? "active" : ""}
                      onClick={() => setMode(m)}
                    >
                      {m}
                    </button>
                  ))}
                </div>
                <input
                  className="compact-input"
                  aria-label="Filter matrix hosts"
                  value={q}
                  maxLength={200}
                  onChange={(e) => {
                    setQ(e.target.value);
                    setHostPage(0);
                  }}
                />
                <button
                  onClick={() =>
                    w.edit("credential_tests", { credential_id: selected })
                  }
                >
                  <Plus size={14} />
                  Record test
                </button>
              </div>
              {mode === "Access matrix" ? (
                <div className="table-scroll">
                  <table className="matrix-table">
                    <thead>
                      <tr>
                        <th>Target host</th>
                        {protocols.map((p) => (
                          <th key={p.name}>{p.name}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {hosts.map((host) => (
                        <tr key={host.id}>
                          <td>
                            <button
                              className="table-name"
                              onClick={() => w.select(host.id)}
                            >
                              {host.name}
                              <small className="mono">{host.ip}</small>
                            </button>
                            {host.service_total > host.services.length && (
                              <button
                                className="matrix-overflow"
                                onClick={() =>
                                  w.edit("credential_tests", {
                                    credential_id: selected,
                                    asset_id: host.id,
                                  })
                                }
                              >
                                {host.services.length} of {host.service_total}{" "}
                                services · search all
                              </button>
                            )}
                          </td>
                          {protocols.map((protocol) => {
                            const services = (host.services as Row[]).filter(
                              (s) =>
                                protocol.name === "Other"
                                  ? !protocols
                                      .slice(0, 4)
                                      .some((p) => p.ports.includes(s.port))
                                  : protocol.ports.includes(s.port),
                            );
                            return (
                              <td key={protocol.name}>
                                {services.length ? (
                                  services.map((service) => {
                                    const test = service;
                                    return (
                                      <button
                                        key={service.id}
                                        className={`matrix-cell ${test?.result === "Valid" ? "valid" : test?.result === "Invalid" ? "invalid" : "untested"}`}
                                        aria-label={`${host.name} ${protocol.name} ${service.port}: ${test?.result || "Untested"}`}
                                        title={`${service.name} :${service.port} · ${test?.result || "Untested"} · Click to record a result`}
                                        onClick={() =>
                                          w.edit("credential_tests", {
                                            credential_id: selected,
                                            asset_id: host.id,
                                            service_id: service.id,
                                          })
                                        }
                                      >
                                        {test?.result === "Valid" ? (
                                          <Check size={16} />
                                        ) : test?.result === "Invalid" ? (
                                          <X size={15} />
                                        ) : (
                                          <Minus size={15} />
                                        )}
                                        <span>{service.port}</span>
                                      </button>
                                    );
                                  })
                                ) : (
                                  <span
                                    className="matrix-absent"
                                    title="No matching service observed"
                                  >
                                    ·
                                  </span>
                                )}
                              </td>
                            );
                          })}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="test-history">
                  {tests.length ? (
                    tests.map((t) => (
                      <button
                        key={t.id}
                        className="history-row"
                        onClick={() => w.select(t.id)}
                      >
                        <Badge>{t.result}</Badge>
                        <strong>{t.asset_name}</strong>
                        <span>{t.service_name}</span>
                        <span>{t.privilege}</span>
                        <time>{time(t.created_at)}</time>
                      </button>
                    ))
                  ) : (
                    <Empty
                      title="No authentication results recorded"
                      detail="Record a validation result for this identity and a known service."
                    />
                  )}
                </div>
              )}
              <div className="paged-controls">
                <span>
                  {mode === "Access matrix"
                    ? matrix.total + " hosts"
                    : matrix.history_total + " recorded tests"}
                </span>
                <button
                  disabled={mode === "Access matrix" ? !hostPage : !historyPage}
                  onClick={() =>
                    mode === "Access matrix"
                      ? setHostPage(hostPage - 1)
                      : setHistoryPage(historyPage - 1)
                  }
                >
                  Previous {mode === "Access matrix" ? "hosts" : "tests"}
                </button>
                <button
                  disabled={
                    mode === "Access matrix"
                      ? (hostPage + 1) * 25 >= matrix.total
                      : (historyPage + 1) * 50 >= matrix.history_total
                  }
                  onClick={() =>
                    mode === "Access matrix"
                      ? setHostPage(hostPage + 1)
                      : setHistoryPage(historyPage + 1)
                  }
                >
                  Next {mode === "Access matrix" ? "hosts" : "tests"}
                </button>
              </div>
              <div className="matrix-legend">
                <span className="good">
                  <Check size={13} />
                  Confirmed valid
                </span>
                <span className="bad">
                  <X size={13} />
                  Confirmed invalid
                </span>
                <span className="warn">
                  <Minus size={13} />
                  Untested
                </span>
                <span>· Service not observed</span>
              </div>
            </div>
          </div>
        </>
      )}
      {reveal && (
        <Modal
          title="Temporarily revealed secret"
          subtitle="This value re-masks after 15 seconds or when the window loses focus."
          onClose={() => setReveal(null)}
        >
          <div className="secret-value mono">{reveal.secret}</div>
          <footer>
            <button onClick={() => setReveal(null)}>Mask now</button>
            <button
              onClick={() =>
                copy(reveal.secret)
                  .then(() => w.notify("Secret copied to clipboard"))
                  .catch((e) => w.notify(e.message, true))
              }
            >
              <Copy size={14} />
              Copy secret
            </button>
          </footer>
        </Modal>
      )}
    </div>
  );
}
export function VaultDialog({ onClose }: { onClose: () => void }) {
  const w = useWorkspace();
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const initialized = w.summary.vault_initialized;
  return (
    <Modal
      title={
        initialized ? "Unlock credential vault" : "Initialize credential vault"
      }
      subtitle="Argon2id key derivation · XChaCha20-Poly1305 authenticated encryption"
      onClose={onClose}
    >
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (!initialized && password !== confirmation) {
            setError("The passphrases do not match");
            return;
          }
          setBusy(true);
          try {
            await w.run("vault_unlock", { password }, "Vault unlocked");
            setPassword("");
            onClose();
          } catch (e) {
            setError(e instanceof Error ? e.message : "Could not unlock vault");
          } finally {
            setBusy(false);
          }
        }}
      >
        {w.summary.engagement.demo === 1 && (
          <p className="demo-callout">
            Demo passphrase: <code>nexus-demo-only</code>
          </p>
        )}
        <label>
          Passphrase
          <input
            type="password"
            autoComplete="current-password"
            minLength={10}
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoFocus
          />
        </label>
        {!initialized && (
          <>
            <label>
              Confirm passphrase
              <input
                type="password"
                autoComplete="new-password"
                minLength={10}
                required
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
              />
            </label>
            <p className="secondary small">
              Keep this passphrase secure. NEXUS cannot recover encrypted
              secrets without it.
            </p>
          </>
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
              ? "Deriving key…"
              : initialized
                ? "Unlock vault"
                : "Initialize vault"}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
