import fs from "node:fs";
import path from "node:path";
import { repository, releaseFingerprint } from "./fingerprint-lib.mjs";

const report = releaseFingerprint(repository);
if (!/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(report.application_version))
  throw Error("Invalid release version");
const directory = path.join(repository, "release-fingerprints");
fs.mkdirSync(directory, { recursive: true });
const filename = "NEXUS-" + report.application_version + ".json";
fs.writeFileSync(
  path.join(directory, filename),
  JSON.stringify(report, null, 2) + "\n",
);
console.log(
  "Wrote ignored local release-fingerprints/" +
    filename +
    " · " +
    report.files.length +
    " first-party files. Nothing was published.",
);
