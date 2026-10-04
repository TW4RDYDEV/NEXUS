// NEXUS — TWARDY.exe / TW4RDYDEV.
// Sanitize a Tauri-generated AppImage by removing host display-stack
// libraries that can conflict with newer Mesa/EGL/Wayland environments,
// then rebuild the AppImage using pinned and hash-verified AppImage tooling.

import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

import { isForbiddenAppImageLibrary } from "./appimage-display-policy.mjs";

const APPIMAGETOOL_URL =
  "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage";

const APPIMAGETOOL_SHA256 =
  "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0";

const APPIMAGE_RUNTIME_URL =
  "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64";

const APPIMAGE_RUNTIME_SHA256 =
  "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d";

const inputArg = process.argv[2];
const outputArg = process.argv[3];

if (!inputArg || !outputArg) {
  console.error(
    "Usage: node scripts/sanitize-appimage.mjs <input-AppImage> <output-AppImage>",
  );
  process.exit(1);
}

if (process.platform !== "linux") {
  console.error("NEXUS AppImage sanitization must run on Linux.");
  process.exit(1);
}

if (process.arch !== "x64") {
  console.error(
    `NEXUS AppImage sanitization currently supports x86_64 only. Detected: ${process.arch}`,
  );
  process.exit(1);
}

const input = path.resolve(inputArg);
const output = path.resolve(outputArg);

if (!fs.existsSync(input)) {
  console.error(`Input AppImage was not found: ${input}`);
  process.exit(1);
}

if (input === output) {
  console.error("Input and output AppImage paths must be different.");
  process.exit(1);
}

const sha256Buffer = (buffer) =>
  crypto.createHash("sha256").update(buffer).digest("hex");

const sha256File = (file) =>
  crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");

const downloadVerified = async (url, destination, expectedSha256) => {
  if (fs.existsSync(destination)) {
    const existingSha256 = sha256File(destination);

    if (existingSha256 === expectedSha256) {
      console.log(`Using cached verified tool: ${destination}`);
      return;
    }

    fs.rmSync(destination, { force: true });
  }

  console.log(`Downloading pinned release tool: ${url}`);

  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(
      `Download failed with HTTP ${response.status}: ${response.statusText}`,
    );
  }

  const data = Buffer.from(await response.arrayBuffer());
  const actualSha256 = sha256Buffer(data);

  if (actualSha256 !== expectedSha256) {
    throw new Error(
      [
        "Downloaded release tool failed SHA-256 verification.",
        `Expected: ${expectedSha256}`,
        `Actual:   ${actualSha256}`,
        `URL:      ${url}`,
      ].join("\n"),
    );
  }

  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, data);
};

const tempRoot = fs.mkdtempSync(
  path.join(os.tmpdir(), "nexus-appimage-sanitize-"),
);

const appDir = path.join(tempRoot, "squashfs-root");

const cacheRoot = path.join(
  process.env.XDG_CACHE_HOME || path.join(os.homedir(), ".cache"),
  "nexus-release-tools",
);

const appImageTool = path.join(cacheRoot, "appimagetool-1.9.1-x86_64.AppImage");

const appImageRuntime = path.join(cacheRoot, "runtime-x86_64-20251108");

const removeForbiddenLibraries = (directory, removed = []) => {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      removeForbiddenLibraries(absolute, removed);
      continue;
    }

    if (isForbiddenAppImageLibrary(entry.name)) {
      removed.push(path.relative(appDir, absolute));
      fs.rmSync(absolute, { force: true });
    }
  }

  return removed;
};

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    ...options,
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    throw new Error(
      `Command failed with exit code ${result.status ?? "unknown"}: ${command}`,
    );
  }
};

try {
  console.log("NEXUS APPIMAGE SANITIZER");
  console.log(`Input:  ${input}`);
  console.log(`Output: ${output}`);
  console.log("");

  fs.chmodSync(input, 0o755);

  run(input, ["--appimage-extract"], {
    cwd: tempRoot,
    env: process.env,
  });

  if (!fs.existsSync(appDir)) {
    throw new Error(`Extracted AppDir was not found at ${appDir}.`);
  }

  const removed = removeForbiddenLibraries(appDir).sort();

  if (removed.length === 0) {
    console.log(
      "[OK] No forbidden display-stack libraries were found. Repack is unnecessary.",
    );

    fs.mkdirSync(path.dirname(output), { recursive: true });
    fs.copyFileSync(input, output);
    fs.chmodSync(output, 0o755);
    process.exitCode = 0;
  } else {
    console.log("Removed forbidden host display-stack libraries:");
    console.log("");

    for (const file of removed) {
      console.log(`  ${file}`);
    }

    console.log("");
    console.log(`Removed ${removed.length} library entries.`);
    console.log("");

    await downloadVerified(APPIMAGETOOL_URL, appImageTool, APPIMAGETOOL_SHA256);

    await downloadVerified(
      APPIMAGE_RUNTIME_URL,
      appImageRuntime,
      APPIMAGE_RUNTIME_SHA256,
    );

    fs.chmodSync(appImageTool, 0o755);

    fs.mkdirSync(path.dirname(output), { recursive: true });
    fs.rmSync(output, { force: true });

    console.log("");
    console.log("Rebuilding sanitized AppImage...");

    run(appImageTool, ["--runtime-file", appImageRuntime, appDir, output], {
      env: {
        ...process.env,
        APPIMAGE_EXTRACT_AND_RUN: "1",
        ARCH: "x86_64",
      },
    });

    if (!fs.existsSync(output)) {
      throw new Error(`Sanitized AppImage was not created: ${output}`);
    }

    fs.chmodSync(output, 0o755);

    console.log("");
    console.log("[OK] Sanitized AppImage created successfully.");
    console.log(`SHA-256: ${sha256File(output)}`);
  }
} catch (error) {
  console.error("");
  console.error("[FAIL] AppImage sanitization failed.");
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true });
}
