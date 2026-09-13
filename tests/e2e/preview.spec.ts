import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

test("capture the running demo and verify compact navigation without page errors", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  test.setTimeout(120000);
  const directory = path.resolve("docs/screenshots");
  fs.mkdirSync(directory, { recursive: true });
  const capture = async (name: string) => {
    await page.screenshot({
      path: path.join(directory, name + ".png"),
      animations: "disabled",
    });
  };
  await page.setViewportSize({ width: 1440, height: 920 });
  await page.goto("/");
  await page.getByRole("button", { name: /Explore demo/ }).click();
  await expect(
    page.getByRole("heading", {
      name: "Helios / Meridian · Internal",
      exact: true,
    }),
  ).toBeVisible();
  await capture("01-overview");
  const nav = (name: string) =>
    page
      .locator(".sidebar")
      .getByRole("button", { name: new RegExp("^" + name) })
      .click();
  await nav("Graph");
  await expect(page.locator(".graph-status")).toContainText("relationships");
  await expect(page.locator(".graph-canvas canvas").first()).toBeVisible();
  // Canvas layout completes asynchronously; wait for the rendering to settle before capture.
  await page.waitForTimeout(1200);
  await capture("02-graph");
  await page.getByText("Keyboard accessible node list").click();
  await page
    .locator(".graph-accessible")
    .getByRole("button", { name: "NX-DEMO-WEB02", exact: true })
    .click();
  await expect(
    page
      .locator(".inspector")
      .getByRole("heading", { name: "NX-DEMO-WEB02", exact: true }),
  ).toBeVisible();
  await page.getByText("Keyboard accessible node list").click();
  await capture("03-inspector");
  await page.getByRole("button", { name: "Close inspector" }).click();
  await nav("Credentials");
  await page.getByRole("button", { name: /svc_archive.*Password/ }).click();
  await expect(
    page.getByRole("button", { name: "NX-DEMO-FILE01 SMB 445: Valid" }),
  ).toBeVisible();
  await capture("04-credentials");
  await nav("Pivots");
  await expect(page.getByText("172.22.40.0/24", { exact: true })).toBeVisible();
  await capture("05-pivots");
  await nav("Snapshots");
  await expect(page.locator(".diff-item").first()).toBeVisible();
  await page.locator(".diff-item summary").first().click();
  await capture("06-snapshots");
  await page.setViewportSize({ width: 1366, height: 768 });
  for (const item of [
    "Overview",
    "Graph",
    "Assets",
    "Credentials",
    "Sessions",
    "Pivots",
    "Findings",
    "Evidence",
    "Assessment",
    "Coverage",
    "Timeline",
    "Snapshots",
    "Scope Guard",
    "Settings",
  ]) {
    await nav(item);
    await expect(page.locator("main")).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
  }
  await nav("Coverage");
  await expect(page.locator(".coverage-table tbody tr").first()).toBeVisible();
  await capture("07-coverage-compact");
  expect(errors).toEqual([]);
});
