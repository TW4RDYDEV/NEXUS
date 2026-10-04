// NEXUS — TWARDY.exe / TW4RDYDEV. Deterministic local Windows release staging.
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    windowsHide: true,
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
};

if (process.platform !== "win32") {
  console.error("NEXUS Windows release builds must be created on Windows.");
  process.exit(1);
}

run(process.execPath, [
  path.join(root, "scripts/release-readiness.mjs"),
  "--require-license",
]);

if (process.env.RUSTFLAGS) {
  console.error(
    "RUSTFLAGS is set. Clear it for a controlled NEXUS release build or migrate required flags to CARGO_ENCODED_RUSTFLAGS.",
  );
  process.exit(1);
}

const remaps = [];
const addRemap = (from, to) => {
  if (!from) return;
  const normalized = path.resolve(from);
  if (
    !remaps.some(
      ([existing]) => existing.toLowerCase() === normalized.toLowerCase(),
    )
  )
    remaps.push([normalized, to]);
};
addRemap(root, "/nexus-src");
addRemap(process.env.CARGO_HOME || path.join(os.homedir(), ".cargo"), "/cargo");
addRemap(
  process.env.RUSTUP_HOME || path.join(os.homedir(), ".rustup"),
  "/rustup",
);
addRemap(os.homedir(), "/user");

const encoded = (process.env.CARGO_ENCODED_RUSTFLAGS || "")
  .split("\x1f")
  .filter(Boolean);
for (const [from, to] of remaps)
  encoded.push(`--remap-path-prefix=${from}=${to}`);

const env = {
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: encoded.join("\x1f"),
  CARGO_INCREMENTAL: "0",
};

let tauriCli;
try {
  tauriCli = require.resolve("@tauri-apps/cli/tauri.js");
} catch {
  console.error("Tauri CLI is not installed. Run `npm ci` first.");
  process.exit(1);
}

const bundle = process.argv.includes("--bundle");
console.log(
  `Building NEXUS with local path remapping enabled${bundle ? " (with configured bundle targets)" : ""}…`,
);
const tauriArgs = ["build", "--features", "desktop"];
if (!bundle) tauriArgs.push("--no-bundle");
run(process.execPath, [tauriCli, ...tauriArgs], { env });

const targetRoot = process.env.CARGO_TARGET_DIR
  ? path.resolve(process.env.CARGO_TARGET_DIR)
  : path.join(root, "src-tauri", "target");
const candidates = [
  path.join(targetRoot, "release", "nexus.exe"),
  path.join(targetRoot, "release", "NEXUS.exe"),
];
const builtExe = candidates.find((candidate) => fs.existsSync(candidate));
if (!builtExe) {
  console.error(
    `Release executable was not found under ${path.join(targetRoot, "release")}.`,
  );
  process.exit(1);
}

const version = JSON.parse(
  fs.readFileSync(path.join(root, "package.json"), "utf8"),
).version;
const stage = path.join(root, "release", "windows-x64");
fs.rmSync(stage, { recursive: true, force: true });
fs.mkdirSync(stage, { recursive: true });
const stagedExe = path.join(stage, "NEXUS.exe");
fs.copyFileSync(builtExe, stagedExe);

if (bundle) {
  const nsisDir = path.join(targetRoot, "release", "bundle", "nsis");
  const expectedInstaller = `NEXUS_${version}_x64-setup.exe`;
  const installerPath = path.join(nsisDir, expectedInstaller);

  if (!fs.existsSync(installerPath)) {
    const available = fs.existsSync(nsisDir)
      ? fs.readdirSync(nsisDir).filter((name) => name.endsWith(".exe"))
      : [];

    console.error(`Expected NSIS installer was not found: ${installerPath}`);

    if (available.length) {
      console.error(`Available NSIS installers: ${available.join(", ")}`);
    }

    process.exit(1);
  }

  fs.copyFileSync(
    installerPath,
    path.join(stage, `NEXUS-Setup-${version}.exe`),
  );
}

const dll = path.join(path.dirname(builtExe), "WebView2Loader.dll");
if (fs.existsSync(dll))
  fs.copyFileSync(dll, path.join(stage, "WebView2Loader.dll"));
fs.writeFileSync(path.join(stage, "portable.flag"), "", "utf8");
for (const [source, destination] of [
  ["LICENSE", "LICENSE.txt"],
  ["docs/THIRD_PARTY_NOTICES.txt", "THIRD_PARTY_NOTICES.txt"],
]) {
  const input = path.join(root, source);
  if (fs.existsSync(input))
    fs.copyFileSync(input, path.join(stage, destination));
}
fs.writeFileSync(
  path.join(stage, "START-HERE.txt"),
  [
    `NEXUS ${version}`,
    "Offensive Security Engagement Platform",
    "by TWARDY.exe",
    "",
    "Keep all files in this directory together.",
    "Run NEXUS.exe on Windows x64 with Microsoft Edge WebView2 Runtime available.",
    "portable.flag keeps application data beside this release in the local `data` directory.",
    "Use NEXUS only on systems and environments you are authorized to assess.",
    "",
  ].join("\r\n"),
  "utf8",
);

run(process.execPath, [
  path.join(root, "scripts/release-readiness.mjs"),
  "--require-license",
  "--binary",
  stagedExe,
]);

const sha256 = (file) =>
  crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const files = fs
  .readdirSync(stage)
  .filter((name) => fs.statSync(path.join(stage, name)).isFile())
  .sort();
const sums = files.map((name) => `${sha256(path.join(stage, name))}  ${name}`);
fs.writeFileSync(
  path.join(stage, "SHA256SUMS.txt"),
  sums.join("\r\n") + "\r\n",
  "utf8",
);
console.log(`NEXUS ${version} staged at ${stage}`);
console.log(`NEXUS.exe SHA-256: ${sha256(stagedExe)}`);
