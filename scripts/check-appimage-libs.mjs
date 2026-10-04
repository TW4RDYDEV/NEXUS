// NEXUS — TWARDY.exe / TW4RDYDEV.
// Reject Linux AppImages that bundle host display-stack libraries known to
// conflict with newer Mesa/EGL/Wayland environments.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const forbiddenLibraries = [
  /^libwayland-client\.so(?:\.|$)/,
  /^libwayland-cursor\.so(?:\.|$)/,
  /^libwayland-egl\.so(?:\.|$)/,
  /^libwayland-server\.so(?:\.|$)/,
  /^libxkbcommon\.so(?:\.|$)/,
  /^libxcb-randr\.so(?:\.|$)/,
  /^libxcb-render\.so(?:\.|$)/,
  /^libxcb-shm\.so(?:\.|$)/,
  /^libXau\.so(?:\.|$)/,
  /^libXdmcp\.so(?:\.|$)/,
];

const appImageArg = process.argv[2];

if (!appImageArg) {
  console.error(
    "Usage: node scripts/check-appimage-libs.mjs <path-to-AppImage>",
  );
  process.exit(1);
}

if (process.platform !== "linux") {
  console.error("NEXUS AppImage library policy checks must run on Linux.");
  process.exit(1);
}

const appImage = path.resolve(appImageArg);

if (!fs.existsSync(appImage)) {
  console.error(`AppImage was not found: ${appImage}`);
  process.exit(1);
}

const tempRoot = fs.mkdtempSync(
  path.join(os.tmpdir(), "nexus-appimage-policy-"),
);

const appDir = path.join(tempRoot, "squashfs-root");

const walk = (directory, found = []) => {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      walk(absolute, found);
      continue;
    }

    if (forbiddenLibraries.some((pattern) => pattern.test(entry.name))) {
      found.push(path.relative(appDir, absolute));
    }
  }

  return found;
};

try {
  console.log("NEXUS APPIMAGE LIBRARY POLICY");
  console.log(`Inspecting: ${appImage}`);
  console.log("");

  const extraction = spawnSync(appImage, ["--appimage-extract"], {
    cwd: tempRoot,
    stdio: "inherit",
    env: process.env,
  });

  if (extraction.error) {
    throw extraction.error;
  }

  if (extraction.status !== 0) {
    console.error(
      `AppImage extraction failed with exit code ${extraction.status ?? "unknown"}.`,
    );
    process.exitCode = 1;
  } else if (!fs.existsSync(appDir)) {
    console.error(`Extracted AppDir was not found at ${appDir}.`);
    process.exitCode = 1;
  } else {
    const violations = walk(appDir).sort();

    if (violations.length > 0) {
      console.error(
        "[FAIL] Forbidden host display-stack libraries were found in the AppImage:",
      );
      console.error("");

      for (const violation of violations) {
        console.error(`  ${violation}`);
      }

      console.error("");
      console.error(
        "AppImage release rejected to prevent Mesa/EGL/Wayland library conflicts.",
      );

      process.exitCode = 1;
    } else {
      console.log(
        "[OK] No forbidden host display-stack libraries are bundled.",
      );
      console.log("AppImage library policy passed.");
    }
  }
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true });
}
