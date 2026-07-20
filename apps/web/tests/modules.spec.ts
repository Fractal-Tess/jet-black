import { expect, test } from "@playwright/test";

import {
  cleanupAccountWithUi,
  createAccountWithUi,
  fillMainIssueTitle,
} from "./helpers/auth";

const MODULES_URL_PATTERN = /\/modules$/;
const MODULE_DETAIL_URL_PATTERN = /\/modules\//;
const ADD_ISSUE_BUTTON_PATTERN = /^Add issue to /;

test.describe("project modules", () => {
  test.describe.configure({ mode: "serial" });

  test.beforeEach(({ page }) => {
    page.setDefaultTimeout(15_000);
  });

  test("creates a module with date range and status, switches layout, and opens detail", async ({
    page,
  }) => {
    const account = await createAccountWithUi(page, {
      name: "Module User",
      prefix: "modules",
    });
    const moduleName = `Module ${crypto.randomUUID().slice(0, 8)}`;

    try {
      await page.getByRole("link", { exact: true, name: "Modules" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      await page.getByRole("button", { name: "New module" }).click();
      await expect(page.getByLabel("Module name")).toBeVisible();

      await page.getByLabel("Module name").fill(moduleName);
      await page.locator("[role='dialog']").getByLabel("Status").click();
      await page.getByText("In progress").last().click();
      await page.getByLabel("Start date").fill("2026-07-01");
      await page.getByLabel("Target date").fill("2026-07-31");
      await page
        .getByLabel("Module description")
        .fill("Module scope created by Playwright.");
      await page.getByRole("button", { name: "Create module" }).click();

      const moduleCard = page
        .locator("article")
        .filter({ hasText: moduleName });
      await expect(moduleCard).toBeVisible();
      await expect(moduleCard).toContainText("In progress");
      await expect(moduleCard).toContainText("2026-07-01 → 2026-07-31");

      // Switch to list layout
      await page.getByRole("button", { name: "List", exact: true }).click();
      await expect(
        page.getByRole("button", { name: "List", exact: true })
      ).toHaveAttribute("aria-pressed", "true");
      await expect(page.locator("a", { hasText: moduleName })).toBeVisible();

      await page.getByRole("button", { name: "Gantt", exact: true }).click();
      await expect(
        page.getByRole("button", { name: "Gantt", exact: true })
      ).toHaveAttribute("aria-pressed", "true");
      await expect(
        page.getByRole("link", {
          name: `Open ${moduleName} module details`,
        })
      ).toBeVisible();

      // Switch back to board
      await page.getByRole("button", { name: "Board", exact: true }).click();
      await expect(
        page.getByRole("button", { name: "Board", exact: true })
      ).toHaveAttribute("aria-pressed", "true");

      // Open detail via card
      await page
        .locator("article")
        .filter({ hasText: moduleName })
        .locator("a")
        .first()
        .click();
      await expect(page).toHaveURL(MODULE_DETAIL_URL_PATTERN);
      await expect(page.getByText(moduleName)).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("adds and removes a work item, creates a module-bound issue, and edits metadata", async ({
    page,
  }) => {
    const account = await createAccountWithUi(page, {
      name: "Module User",
      prefix: "modules",
    });
    const issueTitle = `Module ticket ${crypto.randomUUID()}`;
    const moduleName = `Module ${crypto.randomUUID().slice(0, 8)}`;

    try {
      await fillMainIssueTitle(page, issueTitle);
      await page.getByRole("button", { name: "Create work item" }).click();
      await expect(
        page.getByRole("heading", { name: issueTitle })
      ).toBeVisible();

      await page.getByRole("link", { exact: true, name: "Modules" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      await page.getByRole("button", { name: "New module" }).click();
      await expect(page.getByLabel("Module name")).toBeVisible();

      await page.getByLabel("Module name").fill(moduleName);
      await page.getByRole("button", { name: "Create module" }).click();

      const moduleCard = page
        .locator("article")
        .filter({ hasText: moduleName });
      await expect(moduleCard).toBeVisible();

      // Open detail
      await moduleCard.locator("a").first().click();
      await expect(page).toHaveURL(MODULE_DETAIL_URL_PATTERN);

      // Add existing issue
      await page.getByText("Add existing").click();
      const addExistingIssue = page.getByRole("button", {
        name: `Add ${issueTitle} to module`,
      });
      await addExistingIssue.click();
      await expect(page.getByText("Assigned")).toBeVisible();
      await expect(page.getByText(issueTitle)).toBeVisible();

      // Remove issue
      await page
        .getByRole("button", { name: `Remove ${issueTitle} from module` })
        .click();
      await expect(addExistingIssue).toBeVisible();

      // Create module-bound issue via quick add in board
      const boundIssueTitle = `Bound ${crypto.randomUUID().slice(0, 8)}`;
      await page
        .getByLabel("Quick issue title for")
        .first()
        .fill(boundIssueTitle);
      await page
        .getByRole("button", { name: ADD_ISSUE_BUTTON_PATTERN })
        .first()
        .click();
      await expect(
        page.getByRole("button").filter({ hasText: boundIssueTitle }).first()
      ).toBeVisible();

      // Go back and edit metadata
      await page.getByRole("button", { name: "Back to modules" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      await page.getByRole("button", { name: `Edit ${moduleName}` }).click();
      await page.getByLabel("Module name").fill(`${moduleName} Updated`);
      await page.locator("[role='dialog']").getByLabel("Status").click();
      await page.getByText("Completed", { exact: true }).last().click();
      await page.getByRole("button", { name: "Save changes" }).click();

      await expect(
        page.locator("article").filter({ hasText: `${moduleName} Updated` })
      ).toBeVisible();
      await expect(
        page.locator("article").filter({ hasText: "Completed" })
      ).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("manages links and enforces archive gating", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Module User",
      prefix: "modules",
    });
    const moduleName = `Module ${crypto.randomUUID().slice(0, 8)}`;

    try {
      await page.getByRole("link", { exact: true, name: "Modules" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      await page.getByRole("button", { name: "New module" }).click();
      await expect(page.getByLabel("Module name")).toBeVisible();

      await page.getByLabel("Module name").fill(moduleName);
      await page.getByRole("button", { name: "Create module" }).click();

      const moduleCard = page
        .locator("article")
        .filter({ hasText: moduleName });
      await expect(moduleCard).toBeVisible();

      // Open detail
      await moduleCard.locator("a").first().click();
      await expect(page).toHaveURL(MODULE_DETAIL_URL_PATTERN);

      // Add link
      await page.getByLabel("New link title").fill("Docs");
      await page.getByLabel("New link URL").fill("https://example.com/docs");
      await page.getByRole("button", { name: "Add link" }).click();
      await expect(page.getByRole("link", { name: "Docs" })).toBeVisible();

      // Copy link
      await page.getByRole("button", { name: "Copy link Docs" }).click();

      // Edit link
      await page.getByRole("button", { name: "Edit link Docs" }).click();
      await page
        .getByRole("textbox", { name: "Link title", exact: true })
        .fill("Documentation");
      await page.getByRole("button", { name: "Save" }).click();
      await expect(page.getByText("Documentation")).toBeVisible();

      // Delete link
      await page
        .getByRole("button", { name: "Delete link Documentation" })
        .click();
      await expect(page.getByText("Documentation")).not.toBeVisible();

      // Archive should be disabled for non-completed module
      const archiveButton = page.getByRole("button", {
        name: "Archive module",
      });
      await expect(archiveButton).toBeDisabled();

      // Go back, edit to completed, then archive
      await page.getByRole("button", { name: "Back to modules" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      await page.getByRole("button", { name: `Edit ${moduleName}` }).click();
      await page.locator("[role='dialog']").getByLabel("Status").click();
      await page.getByText("Completed", { exact: true }).last().click();
      await page.getByRole("button", { name: "Save changes" }).click();

      await moduleCard.locator("a").first().click();
      await expect(page).toHaveURL(MODULE_DETAIL_URL_PATTERN);

      await archiveButton.click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      // Show archived and restore
      await page.getByRole("button", { name: "Show archived" }).click();
      await expect(
        page.locator("article").filter({ hasText: moduleName })
      ).toBeVisible();

      await page
        .locator("article")
        .filter({ hasText: moduleName })
        .locator("a")
        .first()
        .click();
      await expect(
        page.getByRole("button", { name: "Restore module" })
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: "Edit module" })
      ).not.toBeVisible();
      await expect(
        page.getByRole("button", { name: "Add link" })
      ).not.toBeVisible();
      await page.getByRole("button", { name: "Restore module" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      await expect(
        page.locator("article").filter({ hasText: moduleName })
      ).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
