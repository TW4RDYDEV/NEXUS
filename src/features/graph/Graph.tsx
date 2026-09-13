import { useEffect, useRef, useState } from "react";
import cytoscape, {
  type Core,
  type ElementDefinition,
  type StylesheetJson,
} from "cytoscape";
import {
  Plus,
  Minus,
  Maximize,
  Focus,
  RotateCcw,
  LayoutGrid,
  Route,
  Search,
  Layers,
  Camera,
  Link2,
} from "lucide-react";
import { api, type Row } from "../../lib/api";
import { useWorkspace } from "../../state/workspace";
import { PageTitle, Badge, Loading, Modal } from "../../components/ui";
const baseLayers = [
  "Operator",
  "Host",
  "Network",
  "Domain",
  "Web Application",
  "User",
  "Credential",
  "Session",
  "Pivot",
];
const styles: StylesheetJson = [
  {
    selector: "node",
    style: {
      "background-color": "#171e25",
      "border-color": "#3a4652",
      "border-width": 1,
      shape: "round-rectangle",
      width: 156,
      height: 68,
      label: "data(display)",
      color: "#e8edf1",
      "font-family": "Segoe UI, sans-serif",
      "font-size": 14,
      "text-wrap": "wrap",
      "text-max-width": "145",
      "text-valign": "center",
      "text-halign": "center",
      "overlay-opacity": 0,
    },
  },
  {
    selector: 'node[kind="Host"]',
    style: {
      width: 184,
      height: 88,
      "background-color": "#141b21",
      "border-color": "#526170",
      "border-width": 1.2,
      "font-size": 14,
    },
  },
  {
    selector: 'node[kind="Operator"]',
    style: {
      shape: "ellipse",
      width: 100,
      height: 100,
      "background-color": "#17232c",
      "border-color": "#7ba4ba",
    },
  },
  {
    selector: 'node[kind="Credential"]',
    style: {
      shape: "round-rectangle",
      height: 54,
      width: 140,
      "background-color": "#211f19",
      "border-color": "#6d6042",
      color: "#dcd1b3",
    },
  },
  {
    selector: 'node[kind="Session"]',
    style: {
      height: 58,
      "background-color": "#17231f",
      "border-color": "#436858",
    },
  },
  {
    selector: 'node[kind="Network"]',
    style: {
      shape: "round-rectangle",
      height: 48,
      "border-style": "dashed",
      "background-color": "#13171c",
      color: "#a8b4c0",
    },
  },
  {
    selector: 'node[kind="Pivot"]',
    style: {
      shape: "diamond",
      height: 96,
      width: 158,
      "background-color": "#19212b",
      "border-color": "#596e88",
    },
  },
  {
    selector: 'node[kind="Service"]',
    style: {
      height: 48,
      width: 130,
      "background-color": "#14181d",
      "font-size": 10,
    },
  },
  {
    selector: 'node[kind="Finding"]',
    style: {
      height: 64,
      width: 170,
      "background-color": "#281e20",
      "border-color": "#7e5359",
      "font-size": 10,
    },
  },
  {
    selector: "edge",
    style: {
      width: 1.2,
      "line-color": "#3c4853",
      "target-arrow-color": "#536474",
      "target-arrow-shape": "triangle",
      "curve-style": "bezier",
      label: "data(label)",
      "font-size": 10,
      color: "#8495a5",
      "text-rotation": "autorotate",
      "text-background-color": "#0d1116",
      "text-background-opacity": 1,
      "text-background-padding": "3",
      "arrow-scale": 0.7,
      "overlay-opacity": 0,
    },
  },
  { selector: ".dim", style: { opacity: 0.12 } },
  {
    selector: ".path",
    style: {
      "border-color": "#8ab79d",
      "border-width": 2,
      "line-color": "#8ab79d",
      "target-arrow-color": "#8ab79d",
      width: 2,
    },
  },
  { selector: "node.path", style: { width: 184 } },
  {
    selector: ".changed",
    style: { "border-color": "#d3b675", "border-width": 2 },
  },
  {
    selector: ".change-new",
    style: { "border-color": "#8fbf9e", "border-width": 2 },
  },
  {
    selector: ".change-removed",
    style: {
      "border-color": "#c98993",
      "border-width": 2,
      "border-style": "dashed",
      opacity: 0.7,
    },
  },
  {
    selector: "node:selected",
    style: {
      "border-color": "#a6d0e5",
      "border-width": 2.5,
      "background-color": "#202e3b",
    },
  },
  {
    selector: ":parent",
    style: {
      "background-color": "#12171d",
      "background-opacity": 0.5,
      "border-color": "#2d3640",
      "border-style": "dashed",
      padding: "24",
      label: "data(name)",
      "text-valign": "top",
      "text-halign": "center",
      "font-size": 12,
      color: "#8b9ba9",
    },
  },
];
export function Graph() {
  const w = useWorkspace();
  const container = useRef<HTMLDivElement>(null);
  const cy = useRef<Core | null>(null);
  const [data, setData] = useState<Row | null>(null);
  const [layers, setLayers] = useState(baseLayers);
  const [showLayers, setShowLayers] = useState(false);
  const [mode, setMode] = useState(w.changed.length ? "Changed" : "Assets");
  const [buildingPath, setBuildingPath] = useState(false);
  const [manualPath, setManualPath] = useState<string[]>(["operator"]);
  const [layout, setLayout] = useState("breadthfirst");
  const [assetPage, setAssetPage] = useState(0);
  const [browse, setBrowse] = useState("");
  const [query, setQuery] = useState("");
  const [group, setGroup] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [error, setError] = useState("");
  const selectRef = useRef(w.select);
  selectRef.current = w.select;
  useEffect(() => {
    let active = true;
    setError("");
    api("graph", {
      include_path: mode === "Attack Path",
      layers,
      focus: w.graphFocus,
      q: browse,
      offset: assetPage * 60,
    })
      .then((d) => {
        if (active) setData(d);
      })
      .catch((e) => setError(e.message));
    return () => {
      active = false;
    };
  }, [w.revision, layers, w.graphFocus, assetPage, browse, mode]);
  useEffect(() => {
    if (!container.current || !data) return;
    const nodes: Row[] = [...data.nodes];
    if (mode === "Changed")
      for (const change of w.changed.slice(0, 500)) {
        if (!nodes.some((n) => n.id === change.entity_id)) {
          const row = change.after || change.before;
          const kind: Record<string, string> = {
            assets: row.kind || "Host",
            services: "Service",
            sessions: "Session",
            pivots: "Pivot",
            findings: "Finding",
          };
          if (kind[change.kind])
            nodes.push({
              id: change.entity_id,
              name: change.name,
              kind: kind[change.kind],
              subtitle: row.ip || row.hostname || row.network || change.status,
            });
        }
      }
    const elements: ElementDefinition[] = [];
    const groups = new Set<string>();
    for (const n of nodes) {
      let parent = "";
      if (group && n.kind === "Host" && n.subtitle?.match(/^\d+\./)) {
        parent = "group-" + n.subtitle.split(".").slice(0, 3).join(".");
        if (!groups.has(parent)) {
          groups.add(parent);
          elements.push({
            data: {
              id: parent,
              name: n.subtitle.split(".").slice(0, 3).join(".") + ".0/24",
              kind: "Group",
            },
          });
        }
      }
      elements.push({
        data: {
          ...n,
          display: `${n.name}\n${n.subtitle || ""}${n.kind === "Host" && n.detail ? "\n" + n.detail : ""}`,
          parent,
        },
      });
    }
    for (const e of data.edges)
      elements.push({
        data: { ...e, label: e.label },
      });
    const instance = cytoscape({
      container: container.current,
      elements,
      style: styles,
      layout: {
        name: layout,
        directed: true,
        roots: "#operator",
        padding: 40,
        spacingFactor: 1,
        ...(layout === "breadthfirst"
          ? {
              boundingBox: {
                x1: 0,
                y1: 0,
                w: container.current.clientHeight,
                h: container.current.clientWidth,
              },
              transform: (
                _node: unknown,
                position: { x: number; y: number },
              ) => ({ x: position.y, y: position.x }),
            }
          : {}),
        animate: false,
      } as any,
      minZoom: 0.18,
      maxZoom: 2.3,
      selectionType: "single",
    });
    cy.current = instance;
    instance.on("tap", "node", (event) => {
      if (
        event.target.data("kind") !== "Group" &&
        !String(event.target.id()).startsWith("network:") &&
        event.target.id() !== "operator"
      )
        selectRef.current(event.target.id());
    });
    instance.on("cxttap", (event) =>
      setMenu({ x: event.renderedPosition.x, y: event.renderedPosition.y }),
    );
    instance.on("tap", () => setMenu(null));
    const observer = new ResizeObserver(() => instance.resize());
    observer.observe(container.current);
    return () => {
      observer.disconnect();
      instance.destroy();
      cy.current = null;
    };
  }, [data, layout, group, mode === "Changed", w.changed]);
  useEffect(() => {
    const c = cy.current;
    if (!c) return;
    c.elements().removeClass("dim path changed change-new change-removed");
    const path = w.summary.path as string[];
    if (mode === "Attack Path" && path.length) {
      c.elements().addClass("dim");
      path.forEach((id) =>
        c.getElementById(id).removeClass("dim").addClass("path"),
      );
      c.edges().forEach((e) => {
        if (
          path.some(
            (p, i) => p === e.source().id() && path[i + 1] === e.target().id(),
          )
        )
          e.removeClass("dim").addClass("path");
      });
    } else if (mode === "Changed") {
      c.elements().addClass("dim");
      w.changed.forEach((change) =>
        c
          .getElementById(change.entity_id)
          .removeClass("dim")
          .addClass(
            change.status === "New"
              ? "change-new"
              : change.status === "Removed"
                ? "change-removed"
                : "changed",
          ),
      );
    } else if (mode === "Untested") {
      c.elements().addClass("dim");
      for (const opportunity of w.summary.opportunities)
        c.getElementById(opportunity.entity_id).removeClass("dim");
    }
    if (query) {
      c.nodes().forEach((n) => {
        if (
          !`${n.data("name")} ${n.data("subtitle")}`
            .toLowerCase()
            .includes(query.toLowerCase())
        )
          n.addClass("dim");
        else n.removeClass("dim");
      });
    }
  }, [mode, data, w.summary.path, w.changed, query, group, layout]);
  useEffect(() => {
    const c = cy.current;
    if (!c) return;
    c.nodes().unselect();
    if (w.selected) c.getElementById(w.selected).select();
  }, [w.selected, data]);
  const switchMode = (value: string) => {
    setMode(value);
    if (value === "Credentials") setLayers([...baseLayers, "Service"]);
    else if (value === "Findings") setLayers([...baseLayers, "Finding"]);
    else if (value === "Untested")
      setLayers([...baseLayers, "Service", "Finding"]);
    else if (value === "Access")
      setLayers([
        "Operator",
        "Host",
        "Credential",
        "Session",
        "Pivot",
        "Network",
      ]);
    else setLayers(baseLayers);
  };
  return (
    <div className="graph-page">
      <PageTitle
        eyebrow="RELATIONSHIP INTELLIGENCE"
        title="Asset graph"
        description="Known assets. Confirmed access. A traceable path."
        actions={
          <>
            <button
              onClick={() => {
                setManualPath(["operator"]);
                setBuildingPath(true);
              }}
            >
              <Route size={14} />
              Pin path
            </button>
            <button onClick={() => w.edit("relationships")}>
              <Link2 size={14} />
              Relationship
            </button>
            <button onClick={() => w.edit("assets")}>
              <Plus size={14} />
              Asset
            </button>
          </>
        }
      />
      <div className="graph-browse">
        <input
          aria-label="Find assets across engagement"
          placeholder="Find assets across the engagement…"
          value={browse}
          maxLength={200}
          onChange={(e) => {
            setBrowse(e.target.value);
            setAssetPage(0);
            if (w.graphFocus) w.focus("");
          }}
        />
        <span>
          {w.graphFocus
            ? "Focused neighborhood"
            : (data?.asset_total || 0) +
              " matching assets · page " +
              (assetPage + 1)}
        </span>
        {w.graphFocus ? (
          <button onClick={() => w.focus("")}>Browse all assets</button>
        ) : (
          <>
            <button
              disabled={assetPage === 0}
              onClick={() => setAssetPage(assetPage - 1)}
            >
              Previous assets
            </button>
            <button
              disabled={!data || (assetPage + 1) * 60 >= data.asset_total}
              onClick={() => setAssetPage(assetPage + 1)}
            >
              Next assets
            </button>
          </>
        )}
      </div>
      <div className="graph-toolbar">
        <div className="segmented">
          {[
            "Assets",
            "Access",
            "Credentials",
            "Findings",
            "Attack Path",
            "Changed",
            "Untested",
          ].map((m) => (
            <button
              key={m}
              className={mode === m ? "active" : ""}
              onClick={() => switchMode(m)}
            >
              {m}
            </button>
          ))}
        </div>
        <button
          className={showLayers ? "active" : ""}
          onClick={() => setShowLayers(!showLayers)}
        >
          <Layers size={14} />
          Layers
        </button>
      </div>
      <div className="graph-canvas-wrap">
        {!data && !error && <Loading label="Building relationship graph" />}
        {error && <div className="error-box">{error}</div>}
        <div
          ref={container}
          className="graph-canvas"
          role="img"
          aria-label="Interactive asset relationship graph"
        />
        <div className="graph-search">
          <Search size={14} />
          <input
            aria-label="Search graph"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <span>{data?.nodes.length ?? 0} nodes</span>
        </div>
        {showLayers && (
          <div className="layer-panel">
            <strong>Visible entities</strong>
            {[...baseLayers, "Service", "Finding"].map((l) => (
              <label key={l}>
                <input
                  type="checkbox"
                  checked={layers.includes(l)}
                  onChange={() =>
                    setLayers(
                      layers.includes(l)
                        ? layers.filter((x) => x !== l)
                        : [...layers, l],
                    )
                  }
                />
                {l}
              </label>
            ))}
            <label>
              <input
                type="checkbox"
                checked={group}
                onChange={(e) => setGroup(e.target.checked)}
              />
              Group hosts by /24
            </label>
          </div>
        )}
        <div className="graph-controls">
          <button
            title="Zoom in"
            aria-label="Zoom in"
            onClick={() => cy.current?.zoom(cy.current.zoom() * 1.2)}
          >
            <Plus size={16} />
          </button>
          <button
            title="Zoom out"
            aria-label="Zoom out"
            onClick={() => cy.current?.zoom(cy.current.zoom() / 1.2)}
          >
            <Minus size={16} />
          </button>
          <i />
          <button
            title="Fit graph"
            aria-label="Fit graph"
            onClick={() => cy.current?.fit(undefined, 50)}
          >
            <Maximize size={16} />
          </button>
          <button
            title="Center selection"
            aria-label="Center selection"
            onClick={() => {
              const c = cy.current;
              if (c && w.selected) c.center(c.getElementById(w.selected));
            }}
          >
            <Focus size={16} />
          </button>
          <button
            title="Change layout"
            aria-label="Change layout"
            onClick={() =>
              setLayout(layout === "breadthfirst" ? "cose" : "breadthfirst")
            }
          >
            <LayoutGrid size={16} />
          </button>
          <button
            title="Reset graph"
            aria-label="Reset graph"
            onClick={() => {
              setQuery("");
              switchMode("Assets");
              setGroup(false);
              setLayout("breadthfirst");
              w.focus("");
            }}
          >
            <RotateCcw size={16} />
          </button>
        </div>
        <div className="graph-legend">
          <span>
            <i className="legend-host" />
            Asset
          </span>
          <span>
            <i className="legend-credential" />
            Credential
          </span>
          <span>
            <i className="legend-session" />
            Session
          </span>
          <span>Arrows follow recorded relationships</span>
        </div>
        {menu && (
          <div
            className="context-menu graph-context"
            style={{ left: Math.min(menu.x, 600), top: Math.min(menu.y, 400) }}
          >
            <button
              onClick={() => {
                cy.current?.fit(undefined, 50);
                setMenu(null);
              }}
            >
              Fit graph
            </button>
            <button
              onClick={() => {
                w.edit("relationships");
                setMenu(null);
              }}
            >
              Create relationship
            </button>
            <button
              onClick={() => {
                w.navigate("Snapshots");
                setMenu(null);
              }}
            >
              Create snapshot
            </button>
          </div>
        )}
        {mode === "Attack Path" && !(w.summary.path?.length > 0) && (
          <div className="graph-note">
            <Route size={18} />
            Select a host and calculate a confirmed access path in its
            inspector.
          </div>
        )}
        {mode === "Changed" && !w.changed.length && (
          <div className="graph-note">
            <Camera size={18} />
            Compare snapshots, then choose “Show in graph”.
          </div>
        )}
      </div>
      <div className="graph-status">
        <span>
          <span className="status-dot" />
          Current engagement neighborhood
        </span>
        <span>
          {data?.edges.length ?? 0} relationships
          {data?.total > 500
            ? " · capped at 500 nodes; focus an asset to explore its neighbors"
            : ""}
        </span>
        {w.graphFocus && (
          <button className="text-button" onClick={() => w.focus("")}>
            Clear graph focus
          </button>
        )}
        <Badge>Directed relationships</Badge>
      </div>
      <details className="graph-accessible">
        <summary>Keyboard accessible node list</summary>
        <div>
          {data?.nodes
            .filter(
              (n: Row) => n.id !== "operator" && !n.id.startsWith("network:"),
            )
            .map((n: Row) => (
              <button key={n.id} onClick={() => w.select(n.id)}>
                {n.name}
              </button>
            ))}
        </div>
      </details>
      {buildingPath && (
        <Modal
          title="Pin a confirmed access path"
          subtitle="Each next step follows a confirmed, directed access relationship in the visible graph."
          onClose={() => setBuildingPath(false)}
        >
          <div className="manual-path">
            {manualPath.map((id, i) => (
              <button
                key={i}
                onClick={() => setManualPath(manualPath.slice(0, i + 1))}
              >
                {data?.nodes.find((n: Row) => n.id === id)?.name || id}
              </button>
            ))}
          </div>
          <label>
            Next confirmed step
            <select
              value=""
              onChange={(e) => {
                if (e.target.value)
                  setManualPath([...manualPath, e.target.value]);
              }}
            >
              <option value="">Choose next step</option>
              {data?.edges
                .filter(
                  (e: Row) =>
                    e.source === manualPath.at(-1) &&
                    e.traversable &&
                    !manualPath.includes(e.target),
                )
                .map((e: Row) => (
                  <option key={e.id} value={e.target}>
                    {data.nodes.find((n: Row) => n.id === e.target)?.name ||
                      e.target}{" "}
                    · {e.label}
                  </option>
                ))}
            </select>
          </label>
          <p className="secondary small">
            Select an earlier step to shorten the path. Pivot connectivity alone
            does not establish authenticated access.
          </p>
          <footer>
            <button onClick={() => setBuildingPath(false)}>Cancel</button>
            <button
              className="primary"
              disabled={manualPath.length < 2}
              onClick={() =>
                w
                  .run(
                    "path",
                    { target: manualPath.at(-1), nodes: manualPath, pin: true },
                    "Preferred path pinned",
                  )
                  .then(() => {
                    setBuildingPath(false);
                    setMode("Attack Path");
                  })
                  .catch(() => {})
              }
            >
              Pin selected path
            </button>
          </footer>
        </Modal>
      )}
    </div>
  );
}
