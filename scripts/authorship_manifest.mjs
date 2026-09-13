import fs from "node:fs";
import path from "node:path";
import {
  repository,
  registries,
  read,
  sourceSections,
  gitCommit,
} from "./fingerprint-lib.mjs";

const data = registries(repository);
const { constants: c } = data;
const checks = [
  ["product ID", c.NEXUS_PRODUCT_ID === "nexus.tw4rdy.core"],
  ["graph schema family", c.NEXUS_SCHEMA_FAMILY === "nx-engagement-graph"],
  ["SQLite application ID", c.NEXUS_APPLICATION_ID === 0x4e585553],
  [
    "workspace format",
    c.NEXUS_FORMAT_ID === "nexus-engagement" &&
      read(repository, "src-tauri/src/db/format.rs").includes(
        '"workspace.json"',
      ),
  ],
  [
    "event namespace",
    data.events.length >= 11 &&
      data.events.some((e) => e.id === "nx.authentication.confirmed"),
  ],
  [
    "relationship vocabulary",
    data.relationships.length === 10 &&
      data.relationships.some((e) => e.id === "NX_REACHABLE_VIA"),
  ],
  [
    "error namespace",
    data.errors.length === 12 &&
      data.errors.some((e) => e.code === "NX-PVT-7314"),
  ],
  [
    "migration names",
    data.migrations.length === 3 &&
      data.migrations.every((m) =>
        fs.existsSync(
          path.join(repository, "src-tauri/migrations", m.name + ".sql"),
        ),
      ),
  ],
  [
    "canonical demo fixture",
    data.fixture.fixture_id === "nx-demo-helios-meridian-v1" &&
      data.fixture.identifiers.includes("svc_archive"),
  ],
  [
    "private output exclusions",
    ["authorship-fingerprint-report.json", "release-fingerprints/"].every(
      (name) => read(repository, ".gitignore").includes(name),
    ),
  ],
];
console.log("NEXUS AUTHORSHIP MARKERS");
for (const [name, ok] of checks)
  console.log((ok ? "[OK] " : "[MISSING] ") + name);
if (checks.some(([, ok]) => !ok)) process.exitCode = 1;
else if (process.argv.includes("--report")) {
  const report = {
    report_version: 1,
    purpose:
      "Local maintainer reference; public, removable architectural markers",
    git_commit: gitCommit(repository),
    canonical_identifiers: c,
    schema_metadata: {
      application_id_hex: "0x4E585553",
      application_id_decimal: c.NEXUS_APPLICATION_ID,
      schema_version: c.NEXUS_SCHEMA_VERSION,
      format_version: c.NEXUS_FORMAT_VERSION,
    },
    error_codes: data.errors,
    event_names: data.events,
    relationships: data.relationships,
    migrations: data.migrations,
    fixture_identifiers: {
      fixture_id: data.fixture.fixture_id,
      identifiers: data.fixture.identifiers,
      topology: data.fixture.topology,
    },
    source_sections: sourceSections(repository),
  };
  fs.writeFileSync(
    path.join(repository, "authorship-fingerprint-report.json"),
    JSON.stringify(report, null, 2) + "\n",
  );
  console.log(
    "Wrote ignored local authorship-fingerprint-report.json. Do not distribute it.",
  );
}
