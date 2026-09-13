import { defineConfig } from "@playwright/test";
import path from "node:path";
export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  workers: 1,
  timeout: 60000,
  expect: { timeout: 15000 },
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://127.0.0.1:1420",
    viewport: { width: 1440, height: 1000 },
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    launchOptions: {
      ...(process.env.NEXUS_BROWSER
        ? { executablePath: process.env.NEXUS_BROWSER }
        : {}),
    },
  },
  webServer: {
    command: "npm run dev",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 60000,
    env: {
      NEXUS_BRIDGE:
        process.env.NEXUS_BRIDGE ||
        path.resolve(
          "src-tauri/target/debug/nexus-bridge" +
            (process.platform === "win32" ? ".exe" : ""),
        ),
      NEXUS_DATA_DIR: process.env.NEXUS_DATA_DIR || path.resolve(".nexus/e2e"),
    },
  },
});
