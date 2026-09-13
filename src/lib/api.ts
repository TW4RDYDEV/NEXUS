import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
export type Row = Record<string, any>;
export async function api<T = any>(op: string, args: Row = {}): Promise<T> {
  if (isTauri()) {
    try {
      return await invoke<T>("nexus", { op, args });
    } catch (error) {
      throw nexusError(error);
    }
  }
  if (!import.meta.env.DEV)
    throw new Error("Open NEXUS in the desktop application.");
  const response = await fetch("/__nexus", {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-Nexus-Local": "1" },
    body: JSON.stringify({ op, args }),
  });
  if (!response.headers.get("content-type")?.includes("application/json"))
    throw Error(
      "Start the Rust development bridge or launch with npm run desktop.",
    );
  const data = await response.json();
  if (data.error) throw nexusError(data.error);
  return data.result as T;
}
export async function chooseDirectory(): Promise<string | null> {
  if (!isTauri()) return null;
  const result = await open({ directory: true, multiple: false });
  return typeof result === "string" ? result : null;
}
export async function chooseFile(): Promise<string | null> {
  if (!isTauri()) return null;
  const result = await open({ multiple: false });
  return typeof result === "string" ? result : null;
}
export async function saveFile(name: string): Promise<string | null> {
  if (!isTauri()) return null;
  return save({ defaultPath: name });
}
export function time(value: string) {
  if (!value) return "—";
  const d = new Date(value);
  return isNaN(d.getTime())
    ? value
    : d.toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
}
export function label(row: Row) {
  return row.name || row.username || row.rule || row.kind || row.id;
}
export function title(value: string) {
  return value.replaceAll("_", " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
export async function copy(value: string) {
  await navigator.clipboard.writeText(value);
}
export function download(name: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function nexusError(error: unknown): Error {
  if (error instanceof Error) return error;
  if (
    error &&
    typeof error === "object" &&
    "code" in error &&
    "message" in error
  ) {
    return new Error("[" + String(error.code) + "] " + String(error.message));
  }
  return new Error(
    typeof error === "string"
      ? error
      : "NEXUS could not complete this operation.",
  );
}
