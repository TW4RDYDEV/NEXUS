// NEXUS — TWARDY.exe / TW4RDYDEV. Maintainer tooling only; no network operations.
import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const repository = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
export const normalize = (text) => text.replace(/\r\n?/g, "\n");
export const sha256 = (text) => createHash("sha256").update(text).digest("hex");
export const read = (root, relative) =>
  normalize(fs.readFileSync(path.join(root, relative), "utf8"));

export function gitCommit(root) {
  try {
    const git = (args) =>
      execFileSync("git", args, {
        cwd: root,
        encoding: "utf8",
        windowsHide: true,
        stdio: ["ignore", "pipe", "ignore"],
      }).trim();
    const canonical = (p) => {
      const resolved = fs.realpathSync(p);
      return process.platform === "win32" ? resolved.toLowerCase() : resolved;
    };
    if (canonical(git(["rev-parse", "--show-toplevel"])) !== canonical(root))
      return null;
    const commit = git(["rev-parse", "HEAD"]);
    return /^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(commit) ? commit : null;
  } catch {
    return null;
  }
}

const directories = [
  "src",
  "src-tauri/src",
  "src-tauri/migrations",
  "src-tauri/capabilities",
  "scripts",
  "tests",
  "fixtures",
];
const rootFiles = [
  "package.json",
  "tsconfig.json",
  "vite.config.ts",
  "vitest.config.ts",
  "playwright.config.ts",
  "eslint.config.js",
  "index.html",
  "src-tauri/Cargo.toml",
  "src-tauri/build.rs",
  "src-tauri/tauri.conf.json",
  "docs/AUTHORSHIP.md",
  "docs/architecture/database-format.md",
];
const excluded = new Set([
  "node_modules",
  "target",
  "dist",
  "gen",
  ".git",
  ".nexus",
  "release",
  "release-fingerprints",
  "large_generated_lab",
  "test-results",
  "playwright-report",
  "authorship-fingerprint-report.json",
]);
const extensions =
  /\.(rs|sql|ts|tsx|js|mjs|json|jsonl|xml|txt|css|html|toml|md)$/;

/** Explicit first-party selection; dependencies, lockfiles, binaries and outputs never enter it. */
export function sourceFiles(root) {
  const selected = new Set();
  const walk = (relative) => {
    const absolute = path.join(root, relative);
    if (!fs.existsSync(absolute)) return;
    const stat = fs.lstatSync(absolute);
    if (!stat.isDirectory() || stat.isSymbolicLink()) return;
    for (const item of fs.readdirSync(absolute, { withFileTypes: true })) {
      if (excluded.has(item.name) || item.isSymbolicLink()) continue;
      const next = relative + "/" + item.name;
      if (item.isDirectory()) walk(next);
      else if (item.isFile() && extensions.test(item.name)) selected.add(next);
    }
  };
  directories.forEach(walk);
  for (const file of rootFiles)
    if (
      fs.existsSync(path.join(root, file)) &&
      fs.lstatSync(path.join(root, file)).isFile()
    )
      selected.add(file);
  return [...selected].sort();
}

export function releaseFingerprint(root) {
  const version = JSON.parse(read(root, "package.json")).version;
  const files = sourceFiles(root).map((file) => ({
    path: file,
    sha256: sha256(read(root, file)),
  }));
  return {
    report_version: 1,
    product: "NEXUS",
    application_version: version,
    git_commit: gitCommit(root),
    hash_algorithm: "SHA-256",
    normalization:
      "UTF-8; CRLF/CR to LF; relative forward-slash paths sorted by Unicode code point; no timestamps",
    files,
    source_set_sha256: sha256(JSON.stringify(files)),
  };
}

export function registries(root) {
  const identity = read(root, "src-tauri/src/identity.rs");
  const constants = Object.fromEntries(
    [...identity.matchAll(/pub const (NEXUS_\w+):[^=]+?=\s*([^;]+);/g)].map(
      ([, key, value]) => [
        key,
        value.startsWith('"') ? JSON.parse(value) : Number(value),
      ],
    ),
  );
  const domain = read(root, "src-tauri/src/domain/mod.rs");
  const events = [
    ...domain.matchAll(/(\w+)\s*=>\s*\(\s*"(nx\.[^"]+)",\s*"([^"]+)"\s*\)/g),
  ].map(([, variant, id, label]) => ({ variant, id, label }));
  const relationships = [
    ...domain.matchAll(/(\w+)\s*=>\s*\(\s*"(NX_[^"]+)",\s*"([^"]+)"\s*\)/g),
  ].map(([, variant, id, label]) => ({ variant, id, label }));
  const errors = [
    ...read(root, "src-tauri/src/errors.rs").matchAll(
      /(\w+)\s*=>\s*\(\s*"(NX-[A-Z]+-\d{4})",\s*"([^"]+)"\s*\)/g,
    ),
  ].map(([, variant, code, condition]) => ({ variant, code, condition }));
  const migrations = [
    ...read(root, "src-tauri/src/db/format.rs").matchAll(
      /\((\d+),\s*"(\d{3}_nx_[^"]+)"\)/g,
    ),
  ].map(([, version, name]) => ({ version: Number(version), name }));
  const fixture = JSON.parse(read(root, "fixtures/canonical-nexus-lab.json"));
  return { constants, events, relationships, errors, migrations, fixture };
}

export function sourceSections(root) {
  const specs = [
    [
      "src-tauri/src/identity.rs",
      "format identity",
      "pub const NEXUS_PRODUCT:",
      "pub fn workspace_metadata",
    ],
    [
      "src-tauri/src/domain/mod.rs",
      "domain events",
      "vocabulary!(DomainEvent",
      "vocabulary!(RelationshipKind",
    ],
    [
      "src-tauri/src/core/intelligence.rs",
      "reachability engine",
      "pub fn reachability(",
      "pub fn shortest_path(",
    ],
    [
      "src-tauri/src/core/intelligence.rs",
      "coverage engine",
      "    pub fn coverage_page(",
      "    pub fn opportunities(",
    ],
    [
      "src-tauri/src/core/intelligence.rs",
      "ReconDelta",
      "    fn meaningful_state(",
      "    pub fn summary(",
    ],
    [
      "src-tauri/src/services/imports.rs",
      "entity resolution",
      "    fn resolve_host(",
      "    pub fn import_preview(",
    ],
  ];
  return specs.map(([file, section, start_marker, end_marker]) => {
    const text = read(root, file);
    const start = text.indexOf(start_marker);
    const end = text.indexOf(end_marker, start + start_marker.length);
    if (start < 0 || end < 0)
      throw Error(
        "Source section moved: " +
          section +
          ". Update the documented section selector.",
      );
    return {
      path: file,
      section,
      start_marker,
      end_marker,
      sha256: sha256(text.slice(start, end)),
    };
  });
}
