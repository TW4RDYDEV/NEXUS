import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  sourceFiles,
  releaseFingerprint,
  normalize,
  sha256,
  repository,
  registries,
  sourceSections,
} from "./fingerprint-lib.mjs";

test("fingerprint selection excludes dependencies, outputs and private reports; order and line endings are deterministic", () => {
  const root = fs.mkdtempSync(
    path.join(os.tmpdir(), "nexus-fingerprint-test-"),
  );
  try {
    const write = (p, text) => {
      fs.mkdirSync(path.dirname(path.join(root, p)), { recursive: true });
      fs.writeFileSync(path.join(root, p), text);
    };
    write("package.json", '{"version":"1.0.0"}');
    write("src/z.ts", "z\r\n");
    write("src/a.ts", "a\n");
    for (const p of [
      "node_modules/dependency.ts",
      "dist/bundle.js",
      "release/NEXUS.exe",
      "release-fingerprints/NEXUS-1.0.0.json",
      "authorship-fingerprint-report.json",
      "src/node_modules/dependency.ts",
      "fixtures/large_generated_lab/nmap.xml",
    ])
      write(p, "excluded");
    assert.deepEqual(sourceFiles(root), [
      "package.json",
      "src/a.ts",
      "src/z.ts",
    ]);
    const before = releaseFingerprint(root);
    write("src/z.ts", "z\n");
    assert.deepEqual(before, releaseFingerprint(root));
    write("src/z.ts", "changed\n");
    assert.notEqual(
      before.source_set_sha256,
      releaseFingerprint(root).source_set_sha256,
    );
    assert.equal(sha256(normalize("a\r\nb\r")), sha256("a\nb\n"));
  } finally {
    const resolved = fs.realpathSync(root);
    assert.equal(path.dirname(resolved), fs.realpathSync(os.tmpdir()));
    assert.ok(path.basename(resolved).startsWith("nexus-fingerprint-test-"));
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("maintainer reports extract the actual Rust registries and stable engine sections", () => {
  const data = registries(repository);
  assert.equal(data.constants.NEXUS_APPLICATION_ID, 1314411859);
  assert.equal(data.relationships.length, 10);
  assert.equal(data.errors.length, 12);
  assert.equal(data.migrations.length, 3);
  assert.ok(data.events.some((e) => e.id === "nx.authentication.confirmed"));
  assert.equal(sourceSections(repository).length, 6);
  assert.ok(
    sourceSections(repository).every((s) => /^[a-f0-9]{64}$/.test(s.sha256)),
  );
});
