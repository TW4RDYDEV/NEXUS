import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
test("assessment outcomes, reference search, client report and verified recovery", async ({
  page,
}) => {
  test.setTimeout(120000);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await page.getByRole("button", { name: /Explore demo/ }).click();
  await expect(
    page.getByRole("heading", {
      name: "Helios / Meridian · Internal",
      exact: true,
    }),
  ).toBeVisible();
  const nav = async (name: string) =>
    page
      .locator(".sidebar")
      .getByRole("button", { name: new RegExp("^" + name) })
      .click();
  await nav("Assessment");
  await page
    .getByRole("button", { name: "Add methodology", exact: true })
    .click();
  await page.getByLabel("Methodology", { exact: true }).selectOption("web-api");
  await page.getByLabel("Owner", { exact: true }).fill("Assessment lead");
  await page.getByLabel("Search Assessment target references").fill("WEB02");
  const target = page.getByLabel("Assessment target", { exact: true });
  await expect(
    target.getByRole("option", { name: /NX-DEMO-WEB02/ }),
  ).toHaveCount(1);
  await target.selectOption({
    label: await target
      .getByRole("option", { name: /NX-DEMO-WEB02/ })
      .innerText(),
  });
  await page.getByRole("button", { name: "Add checks", exact: true }).click();
  await expect(page.locator(".assessment-check")).toHaveCount(6);
  await page
    .getByRole("button", { name: "Update check", exact: true })
    .first()
    .click();
  await page.getByLabel("State", { exact: true }).selectOption("Blocked");
  await page
    .getByRole("button", { name: "Apply changes", exact: true })
    .click();
  await expect(page.getByRole("dialog").getByRole("alert")).toContainText(
    "result is required",
  );
  await page
    .getByLabel("Result / reason", { exact: true })
    .fill("Waiting for the authorized secondary test account.");
  await page
    .getByRole("button", { name: "Apply changes", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".assessment-result")).toContainText(
    "secondary test account",
  );
  await page.screenshot({
    path: path.resolve("docs/screenshots/09-assessment.png"),
    animations: "disabled",
  });
  await nav("Settings");
  await page.getByRole("button", { name: "Security", exact: true }).click();
  await page.getByLabel("Include draft findings", { exact: true }).check();
  const pending = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Export client report", exact: true })
    .click();
  const report = await pending;
  const downloaded = await report.path();
  expect(fs.readFileSync(downloaded!, "utf8")).toContain(
    "Unfinished and blocked work",
  );
  await page
    .getByRole("button", { name: "Export complete workspace", exact: true })
    .click();
  await expect(page.locator(".delivery-result")).toContainText(
    "Integrity verified",
  );
  const bundle = await page.locator(".delivery-result .path-text").innerText();
  await page.getByLabel("Workspace bundle path").fill(bundle);
  await page
    .getByRole("button", { name: "Verify bundle", exact: true })
    .click();
  await expect(page.locator(".delivery-result")).toContainText(
    "Integrity verified",
  );
  await page.screenshot({
    path: path.resolve("docs/screenshots/10-delivery.png"),
    animations: "disabled",
  });
  await page
    .getByRole("button", { name: "Recover verified bundle", exact: true })
    .click();
  await expect(page.locator(".delivery-result")).toContainText(
    "Integrity verified",
  );
  await nav("Assessment");
  await expect(page.locator(".assessment-check")).toHaveCount(6);
  await expect(page.locator(".assessment-result")).toContainText(
    "secondary test account",
  );
  expect(errors).toEqual([]);
});
