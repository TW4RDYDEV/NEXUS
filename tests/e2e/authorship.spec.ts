import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

test("canonical relationships, real error display, About and a local-only diagnostic export", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Explore demo/ }).click();
  await expect(
    page.getByRole("heading", {
      name: "Helios / Meridian · Internal",
      exact: true,
    }),
  ).toBeVisible();
  const nav = (name: string) =>
    page
      .locator(".sidebar")
      .getByRole("button", { name: new RegExp("^" + name) })
      .click();
  await nav("Graph");
  await page.getByRole("button", { name: "Relationship", exact: true }).click();
  const relationship = page.getByLabel("Relationship", { exact: true });
  await expect(relationship).toHaveValue("NX_CONNECTS_TO");
  await expect(
    relationship.getByRole("option", { name: "Connects To", exact: true }),
  ).toHaveAttribute("value", "NX_CONNECTS_TO");
  await expect(relationship.getByRole("option")).toHaveCount(9);
  const source = page.getByLabel("Source", { exact: true });
  const target = page.getByLabel("Target", { exact: true });
  const web = (await source
    .getByRole("option", { name: /^NX-DEMO-WEB02/ })
    .first()
    .getAttribute("value"))!;
  const file = (await target
    .getByRole("option", { name: /^NX-DEMO-FILE01/ })
    .first()
    .getAttribute("value"))!;
  await source.selectOption(web);
  await target.selectOption(web);
  await page.getByRole("button", { name: /^Add relationship$/i }).click();
  await expect(page.getByRole("dialog").getByRole("alert")).toContainText(
    "NX-GPH-3802",
  );
  await target.selectOption(file);
  await page.getByRole("button", { name: /^Add relationship$/i }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const graph = await page.evaluate(
    async () =>
      (
        await (
          await fetch("/__nexus", {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-Nexus-Local": "1",
            },
            body: JSON.stringify({ op: "graph", args: {} }),
          })
        ).json()
      ).result,
  );
  expect(
    graph.edges.some(
      (e: { source: string; target: string; kind: string; label: string }) =>
        e.source === web &&
        e.target === file &&
        e.kind === "NX_CONNECTS_TO" &&
        e.label === "Connects To",
    ),
  ).toBe(true);
  await nav("Settings");
  await page.getByRole("button", { name: "About", exact: true }).click();
  await expect(
    page.getByText("nexus.tw4rdy.core", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("TWARDY.exe / TW4RDYDEV", { exact: false }),
  ).toBeVisible();
  await page.setViewportSize({ width: 1440, height: 920 });
  await page.screenshot({
    path: path.resolve("docs/screenshots/08-about.png"),
    animations: "disabled",
  });
  const pending = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Export local build diagnostic" })
    .click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe("NEXUS-build-diagnostic.json");
  const metadata = JSON.parse(
    fs.readFileSync((await download.path())!, "utf8"),
  );
  expect(Object.keys(metadata).sort()).toEqual([
    "application_version",
    "build_mode",
    "format",
    "git_commit",
    "product",
    "product_id",
    "schema_family",
    "schema_version",
  ]);
  expect(metadata.product_id).toBe("nexus.tw4rdy.core");
  expect(metadata.schema_version).toBe(3);
});
