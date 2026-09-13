import { createContext, useContext } from "react";
import type { Row } from "../lib/api";
export type View =
  | "Overview"
  | "Graph"
  | "Assets"
  | "Credentials"
  | "Sessions"
  | "Pivots"
  | "Findings"
  | "Evidence"
  | "Assessment"
  | "Coverage"
  | "Timeline"
  | "Snapshots"
  | "Scope"
  | "Settings";
export type EditorSpec = { table: string; data?: Row };
export type Workspace = {
  summary: Row;
  lookup: Record<string, Row[]>;
  view: View;
  revision: number;
  selected: string | null;
  graphFocus: string;
  changed: Row[];
  setChanged: (changes: Row[]) => void;
  navigate: (view: View) => void;
  select: (id: string) => void;
  focus: (id: string) => void;
  edit: (table: string, data?: Row) => void;
  refresh: () => Promise<void>;
  run: <T = any>(op: string, args?: Row, message?: string) => Promise<T>;
  notify: (message: string, error?: boolean) => void;
  importFile: () => void;
  openVault: () => void;
  close: () => void;
};
export const WorkspaceContext = createContext<Workspace | null>(null);
export function useWorkspace() {
  const ctx = useContext(WorkspaceContext);
  if (!ctx) throw Error("Workspace context unavailable");
  return ctx;
}
export function entityName(lookup: Record<string, Row[]>, id: string) {
  if (id === "operator") return "Operator";
  for (const rows of Object.values(lookup)) {
    const row = rows.find((r) => r.id === id);
    if (row) return row.name || row.username || row.ip || id;
  }
  return id;
}
