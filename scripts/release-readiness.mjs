// NEXUS — TWARDY.exe / TW4RDYDEV. Local release verification only; no network operations.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const requireLicense = process.argv.includes("--require-license");
const binaryIndex = process.argv.indexOf("--binary");
const binary = binaryIndex >= 0 ? process.argv[binaryIndex + 1] : null;
const failures = [];
const warnings = [];
const ok = [];

const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const exists = (relative) => fs.existsSync(path.join(root, relative));
const pass = (message) => ok.push(message);
const fail = (message) => failures.push(message);
const warn = (message) => warnings.push(message);

const pkg = JSON.parse(read("package.json"));
const pkgLock = JSON.parse(read("package-lock.json"));
const tauri = JSON.parse(read("src-tauri/tauri.conf.json"));
const cargo = read("src-tauri/Cargo.toml");
const cargoLock = read("src-tauri/Cargo.lock");
const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const cargoLockVersion = cargoLock.match(
  /\[\[package\]\]\s*\nname = "nexus"\s*\nversion = "([^"]+)"/,
)?.[1];
const versions = [
  ["package.json", pkg.version],
  ["package-lock.json", pkgLock.version],
  ["package-lock root package", pkgLock.packages?.[""]?.version],
  ["Cargo.toml", cargoVersion],
  ["Cargo.lock nexus package", cargoLockVersion],
  ["tauri.conf.json", tauri.version],
];
if (versions.every(([, value]) => value === pkg.version)) {
  pass(`version metadata agrees on ${pkg.version}`);
} else {
  fail(
    "version mismatch: " +
      versions.map(([file, value]) => `${file}=${value ?? "missing"}`).join(", "),
  );
}
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(pkg.version))
  fail(`invalid semantic version: ${pkg.version}`);
if (pkg.private !== true) fail("package.json must remain private=true");
else pass("npm package is private");
if (!/^publish\s*=\s*false$/m.test(cargo))
  fail("Cargo package must remain publish=false");
else pass("Cargo crate publishing is disabled");
if (/^license\s*=\s*"MIT"$/m.test(cargo))
  fail("Cargo.toml still declares the project as MIT");

const staleFiles = [
  "README.md",
  "CHANGELOG.md",
  "docs/AUTHORSHIP.md",
  "docs/PROJECT_SUMMARY.md",
  "docs/VERIFICATION.md",
  "docs/PROFESSIONAL_WORKFLOWS.md",
];
for (const file of staleFiles) {
  if (!exists(file)) continue;
  const text = read(file);
  if (/\b1\.1\.0\b|\bNEXUS 1\.1\b|\b1\.1 walkthrough\b/.test(text))
    fail(`${file} contains stale NEXUS 1.1 release wording`);
  if (/NEXUS source is MIT licensed|Preserved .*MIT license/i.test(text))
    fail(`${file} contains stale project-level MIT wording`);
}

const textRoots = ["src", "src-tauri/src", "scripts", "tests"];
const textExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".mjs", ".json", ".toml", ".md", ".css", ".html"]);
const localPathPattern = /(?:[A-Za-z]:\\Users\\[^\\\r\n]+|\/Users\/[^/\r\n]+|\/home\/[^/\r\n]+)/;
const walk = (relative) => {
  const absolute = path.join(root, relative);
  if (!fs.existsSync(absolute)) return;
  for (const entry of fs.readdirSync(absolute, { withFileTypes: true })) {
    if (["node_modules", "target", "dist", "release"].includes(entry.name)) continue;
    const next = path.join(relative, entry.name);
    if (entry.isDirectory()) walk(next);
    else if (entry.isFile() && textExtensions.has(path.extname(entry.name))) {
      const text = fs.readFileSync(path.join(root, next), "utf8");
      if (localPathPattern.test(text)) fail(`${next} contains an absolute user-home path`);
    }
  }
};
textRoots.forEach(walk);
if (!failures.some((f) => f.includes("absolute user-home path"))) pass("first-party source contains no user-home paths");

const marker = spawnSync(process.execPath, [path.join(root, "scripts/authorship_manifest.mjs")], {
  cwd: root,
  encoding: "utf8",
  windowsHide: true,
});
if (marker.status === 0) pass("authorship marker manifest passes");
else fail("authorship marker manifest failed");

if (requireLicense) {
  if (!exists("LICENSE")) fail("final LICENSE is missing");
  else {
    const license = read("LICENSE");
    if (/Permission is hereby granted, free of charge, to any person obtaining a copy/i.test(license))
      fail("LICENSE appears to still be the MIT license");
    else if (!/NEXUS/i.test(license)) warn("LICENSE does not mention NEXUS; review before publishing");
    else pass("final project LICENSE is present and is not the MIT template");
  }
  if (exists("LICENSE-PENDING.md")) fail("remove LICENSE-PENDING.md after adding the final license");
} else if (exists("LICENSE-PENDING.md")) {
  pass("pre-release licensing guard is present");
} else if (!exists("LICENSE")) {
  warn("neither LICENSE nor LICENSE-PENDING.md exists");
}

if (binary) {
  const absolute = path.resolve(binary);
  if (!fs.existsSync(absolute)) fail(`binary does not exist: ${absolute}`);
  else {
    const data = fs.readFileSync(absolute);
    const needles = new Set();
    for (const value of [root, os.homedir(), process.env.CARGO_HOME, process.env.RUSTUP_HOME]) {
      if (!value) continue;
      for (const variant of [value, value.replaceAll("\\", "/"), value.replaceAll("/", "\\")]) {
        needles.add(variant);
      }
    }
    for (const needle of needles) {
      const utf8 = Buffer.from(needle, "utf8");
      const utf16 = Buffer.from(needle, "utf16le");
      if (data.indexOf(utf8) >= 0 || data.indexOf(utf16) >= 0)
        fail(`binary leaks local build path: ${needle}`);
    }
    for (const markerText of [
      "NEXUS",
      "nexus.tw4rdy.core",
      "nx-engagement-graph",
      "NX_REACHABLE_VIA",
      "nx.authentication.confirmed",
    ]) {
      if (data.indexOf(Buffer.from(markerText, "utf8")) < 0)
        fail(`binary is missing expected product marker: ${markerText}`);
    }
    if (!failures.some((f) => f.startsWith("binary "))) pass("binary path-leak and product-marker checks pass");
  }
}

console.log("NEXUS RELEASE READINESS");
for (const message of ok) console.log(`[OK] ${message}`);
for (const message of warnings) console.log(`[WARN] ${message}`);
for (const message of failures) console.log(`[FAIL] ${message}`);
if (failures.length) {
  console.error(`Release readiness failed with ${failures.length} issue(s).`);
  process.exit(1);
}
console.log(requireLicense ? "Release metadata gate passed." : "Source readiness gate passed.");
