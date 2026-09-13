import { describe, it, expect } from "vitest";
import { schemas } from "../src/lib/schema";
import { title, label, time, nexusError } from "../src/lib/api";
describe("record forms and presentation boundaries", () => {
  it("renders structured IPC errors without losing stable support codes", () => {
    expect(
      nexusError({
        code: "NX-PVT-7314",
        message: "Pivot source host must match its source session",
      }).message,
    ).toBe("[NX-PVT-7314] Pivot source host must match its source session");
    expect(nexusError("Legacy development error").message).toBe(
      "Legacy development error",
    );
    const existing = new Error("Existing error");
    expect(nexusError(existing)).toBe(existing);
  });
  it("does not expose internal ciphertext or file paths as editable form fields", () => {
    for (const fields of Object.values(schemas)) {
      expect(
        fields.some((f) =>
          ["ciphertext", "file_name", "sha256"].includes(f.key),
        ),
      ).toBe(false);
    }
  });
  it("supports every credential type and observation result", () => {
    expect(schemas.credentials.find((f) => f.key === "kind")?.options).toEqual([
      "Password",
      "NTLM hash",
      "API token",
      "SSH private key reference",
      "Generic secret",
      "Other",
    ]);
    expect(
      schemas.credential_tests.find((f) => f.key === "result")?.options,
    ).toEqual(["Valid", "Invalid", "Unknown"]);
  });
  it("requires target references before recording access", () => {
    expect(
      schemas.credential_tests.filter((f) => f.required).map((f) => f.key),
    ).toEqual(
      expect.arrayContaining([
        "credential_id",
        "asset_id",
        "service_id",
        "source",
      ]),
    );
    expect(schemas.pivots.find((f) => f.key === "session_id")?.required).toBe(
      true,
    );
  });
  it("renders absent and malformed date values without inventing timestamps", () => {
    expect(time("")).toBe("—");
    expect(time("unrecorded")).toBe("unrecorded");
    expect(title("first_seen")).toBe("First Seen");
    expect(label({ username: "svc_lab" })).toBe("svc_lab");
  });
});
