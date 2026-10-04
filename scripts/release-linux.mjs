// NEXUS — TWARDY.exe / TW4RDYDEV.
// Deterministic local Linux x86_64 release staging.

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
    ...options,
  });

  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
};

if (process.platform !== "linux") {
  console.error("NEXUS Linux release builds must be created on Linux.");
  process.exit(1);
}

if (process.arch !== "x64") {
  console.error(
    `NEXUS Linux release staging currently supports x86_64 only. Detected: ${process.arch}`,
  );
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

  if (!remaps.some(([existing]) => existing === normalized)) {
    remaps.push([normalized, to]);
  }
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

for (const [from, to] of remaps) {
  encoded.push(`--remap-path-prefix=${from}=${to}`);
}

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

console.log(
  "Building NEXUS Linux x86_64 release with local path remapping enabled…",
);

run(process.execPath, [tauriCli, "build", "--features", "desktop"], { env });

const targetRoot = process.env.CARGO_TARGET_DIR
  ? path.resolve(process.env.CARGO_TARGET_DIR)
  : path.join(root, "src-tauri", "target");

const rawBinary = path.join(targetRoot, "release", "nexus");

if (!fs.existsSync(rawBinary)) {
  console.error(`Release binary was not found at ${rawBinary}.`);
  process.exit(1);
}

run(process.execPath, [
  path.join(root, "scripts/release-readiness.mjs"),
  "--require-license",
  "--binary",
  rawBinary,
]);

const version = JSON.parse(
  fs.readFileSync(path.join(root, "package.json"), "utf8"),
).version;

const bundleRoot = path.join(targetRoot, "release", "bundle");
const stage = path.join(root, "release", "linux-x64");

fs.rmSync(stage, { recursive: true, force: true });
fs.mkdirSync(stage, { recursive: true });

const findBundle = (directory, extension, label) => {
  const absolute = path.join(bundleRoot, directory);

  if (!fs.existsSync(absolute)) {
    console.error(`${label} bundle directory was not found: ${absolute}`);
    process.exit(1);
  }

  const matches = fs
    .readdirSync(absolute)
    .filter((name) => name.toLowerCase().endsWith(extension.toLowerCase()));

  if (matches.length !== 1) {
    console.error(
      `Expected exactly one ${label} artifact under ${absolute}, found ${matches.length}.`,
    );
    process.exit(1);
  }

  return path.join(absolute, matches[0]);
};

const appImage = findBundle("appimage", ".AppImage", "AppImage");
const deb = findBundle("deb", ".deb", "DEB");
const rpm = findBundle("rpm", ".rpm", "RPM");

const stagedAppImage = path.join(stage, `NEXUS-${version}-x86_64.AppImage`);
const stagedDeb = path.join(stage, `NEXUS-${version}-amd64.deb`);
const stagedRpm = path.join(stage, `NEXUS-${version}-x86_64.rpm`);

const artifacts = [
  [appImage, stagedAppImage],
  [deb, stagedDeb],
  [rpm, stagedRpm],
];

for (const [source, destination] of artifacts) {
  fs.copyFileSync(source, destination);
}

fs.chmodSync(stagedAppImage, 0o755);

run(process.execPath, [
  path.join(root, "scripts/check-appimage-libs.mjs"),
  stagedAppImage,
]);

for (const [source, destination] of [
  ["LICENSE", "LICENSE.txt"],
  ["docs/THIRD_PARTY_NOTICES.txt", "THIRD_PARTY_NOTICES.txt"],
]) {
  const input = path.join(root, source);

  if (fs.existsSync(input)) {
    fs.copyFileSync(input, path.join(stage, destination));
  }
}

fs.writeFileSync(
  path.join(stage, "START-HERE.txt"),
  [
    `NEXUS ${version}`,
    "Offensive Security Engagement Platform",
    "by TWARDY.exe / TW4RDYDEV",
    "",
    "Linux x86_64 release.",
    "",
    "AppImage:",
    `  chmod +x NEXUS-${version}-x86_64.AppImage`,
    `  ./NEXUS-${version}-x86_64.AppImage`,
    "",
    "Debian / Ubuntu:",
    `  sudo apt install ./NEXUS-${version}-amd64.deb`,
    "",
    "Fedora / RHEL:",
    `  sudo dnf install ./NEXUS-${version}-x86_64.rpm`,
    "",
    "Arch Linux users should use the AppImage release.",
    "",
    "Nmap and optional external integrations must be installed separately.",
    "Use NEXUS only on systems and environments you are authorized to assess.",
    "",
  ].join("\n"),
  "utf8",
);

const sha256 = (file) =>
  crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");

const publicArtifacts = artifacts.map(([, destination]) => destination);

const sums = publicArtifacts.map(
  (file) => `${sha256(file)}  ${path.basename(file)}`,
);

fs.writeFileSync(
  path.join(stage, "SHA256SUMS.txt"),
  sums.join("\n") + "\n",
  "utf8",
);

console.log("");
console.log(`NEXUS ${version} Linux release staged at:`);
console.log(stage);
console.log("");

for (const file of publicArtifacts) {
  console.log(`${path.basename(file)} SHA-256: ${sha256(file)}`);
}
