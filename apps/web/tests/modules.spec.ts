import { expect, test } from "@playwright/test";

import {
  cleanupAccountWithUi,
  createAccountWithUi,
  fillMainIssueTitle,
} from "./helpers/auth";

const MODULES_URL_PATTERN = /\/modules$/;

test.describe("project modules", () => {
  test("creates a module and assigns a ticket", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Module User",
      prefix: "modules",
    });
    const issueTitle = `Module ticket ${crypto.randomUUID()}`;
    const moduleName = `Module ${crypto.randomUUID().slice(0, 8)}`;

    try {
      await fillMainIssueTitle(page, issueTitle);
      await page
        .getByLabel("Issue description")
        .fill("Created before assigning to a module.");
      await page.getByRole("button", { name: "Create issue" }).click();
      await expect(
        page.getByRole("heading", { name: issueTitle })
      ).toBeVisible();

      await page.getByRole("link", { exact: true, name: "Modules" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);

      await page.getByLabel("Module name").fill(moduleName);
      await page.getByLabel("Status").selectOption("in_progress");
      await page.getByLabel("Target date").fill("2026-07-31");
      await page
        .getByLabel("Module description")
        .fill("Module scope created by Playwright.");
      await page.getByRole("button", { name: "Create module" }).click();

      const moduleCard = page
        .locator("article")
        .filter({ hasText: moduleName });
      await expect(moduleCard).toBeVisible();
      const ticketSelect = moduleCard.getByLabel("Add ticket to module");
      const ticketOption = ticketSelect
        .locator("option")
        .filter({ hasText: issueTitle })
        .first();
      const ticketOptionValue = await ticketOption.getAttribute("value");

      expect(ticketOptionValue).not.toBeNull();
      await ticketSelect.selectOption(ticketOptionValue ?? "");

      await expect(moduleCard).toContainText(issueTitle);
      await expect(moduleCard).toContainText("0 / 1 tickets done");
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
