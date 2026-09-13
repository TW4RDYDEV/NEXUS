import { test, expect, type Page } from "@playwright/test";
import path from "node:path";
const errors: string[] = [];
test.beforeEach(async ({ page }) => {
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Create engagement", exact: true }),
  ).toBeVisible();
});
test.afterEach(() => {
  expect(errors.splice(0)).toEqual([]);
});
async function demo(page: Page) {
  await page.getByRole("button", { name: /Explore demo/ }).click();
  await expect(
    page.getByRole("heading", {
      name: "Helios / Meridian · Internal",
      exact: true,
    }),
  ).toBeVisible();
}
test("create engagement, import a fixture, edit inspector notes, and persist across reopen", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Create engagement", exact: true })
    .click();
  await page.getByLabel("Engagement name").fill("E2E · Small lab");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Create engagement", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "E2E · Small lab", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Import observations", exact: true })
    .first()
    .click();
  await page
    .locator("input[type=file]")
    .setInputFiles(path.resolve("fixtures/small_lab/nmap.xml"));
  await page.getByRole("button", { name: "Preview import" }).click();
  await expect(
    page.getByRole("heading", { name: "Import preview" }),
  ).toBeVisible();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Import observations", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page
    .getByRole("navigation")
    .getByRole("button", { name: /^Assets/ })
    .click();
  await expect(
    page.getByRole("button", { name: /NX-DEMO-WEB02 nx-demo-web02/ }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: /NX-DEMO-WEB02 nx-demo-web02/ })
    .click();
  await expect(
    page
      .locator(".inspector")
      .getByRole("heading", { name: "NX-DEMO-WEB02", exact: true }),
  ).toBeVisible();
  await page
    .getByLabel("Analyst notes")
    .fill("Verified persistence from the inspector.");
  await page.getByRole("heading", { name: "Host details" }).click();
  await expect(page.getByRole("status")).toContainText("Changes saved");
  await page.locator(".engagement-switcher").click();
  await page
    .getByRole("button", { name: /E2E · Small lab Active/ })
    .first()
    .click();
  await page
    .getByRole("navigation")
    .getByRole("button", { name: /^Assets/ })
    .click();
  await page
    .getByRole("button", { name: /NX-DEMO-WEB02 nx-demo-web02/ })
    .click();
  await expect(page.getByLabel("Analyst notes")).toHaveValue(
    "Verified persistence from the inspector.",
  );
});
test("demo navigation, graph selection, matrix, and snapshots are backed by the core", async ({
  page,
}) => {
  await demo(page);
  await page
    .getByRole("navigation")
    .getByRole("button", { name: "Graph", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Asset graph" }),
  ).toBeVisible();
  await expect(page.locator(".graph-status")).toContainText("relationships");
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
  await page.getByRole("button", { name: "Close inspector" }).click();
  await page
    .getByRole("navigation")
    .getByRole("button", { name: /^Credentials/ })
    .click();
  await page.getByRole("button", { name: /svc_archive.*Password/ }).click();
  await expect(
    page.getByRole("button", { name: "NX-DEMO-FILE01 SMB 445: Valid" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "NX-DEMO-FILE01 WinRM 5985: Invalid" }),
  ).toBeVisible();
  await page
    .getByRole("navigation")
    .getByRole("button", { name: "Snapshots", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Snapshots & changes" }),
  ).toBeVisible();
  await expect(page.locator(".diff-item").first()).toBeVisible();
  await page.locator(".diff-item summary").first().click();
  await expect(page.locator(".diff-fields").first()).toBeVisible();
  await page
    .getByRole("navigation")
    .getByRole("button", { name: "Pivots", exact: true })
    .click();
  await expect(page.getByText("172.22.40.0/24", { exact: true })).toBeVisible();
});
test("vault stays locked until unlocked; authentication updates the matrix", async ({
  page,
}) => {
  await demo(page);
  await page
    .getByRole("navigation")
    .getByRole("button", { name: /^Credentials/ })
    .click();
  await page.getByRole("button", { name: "Reveal", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Unlock credential vault" }),
  ).toBeVisible();
  await page.getByLabel("Passphrase", { exact: true }).fill("nexus-demo-only");
  await page.getByRole("button", { name: "Unlock vault", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Add credential", exact: true })
    .click();
  await page.getByLabel("Username / identifier").fill("e2e_analyst");
  await page.getByLabel("Secret", { exact: true }).fill("FICTIONAL-E2E-ONLY");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Add credential", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: /e2e_analyst.*Password/ }).click();
  await page
    .getByRole("button", { name: "NX-DEMO-WEB02 SSH 22: Untested" })
    .click();
  await page.getByLabel("Authentication result").selectOption("Invalid");
  await page.getByRole("button", { name: "Add authentication test" }).click();
  await expect(
    page.getByRole("button", { name: "NX-DEMO-WEB02 SSH 22: Invalid" }),
  ).toBeVisible();
});
test("keyboard command palette finds an asset and opens its inspector", async ({
  page,
}) => {
  await demo(page);
  await page.keyboard.press("Control+k");
  await page.getByLabel("Search everything").fill("172.22.20.12");
  await expect(page.getByRole("option").first()).toContainText(
    "NX-DEMO-FILE01",
  );
  await page.getByLabel("Search everything").press("Enter");
  await expect(
    page
      .locator(".inspector")
      .getByRole("heading", { name: "NX-DEMO-FILE01", exact: true }),
  ).toBeVisible();
});
