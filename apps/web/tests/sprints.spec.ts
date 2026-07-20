import { expect, test } from "@playwright/test";

import {
  cleanupAccountWithUi,
  createAccountWithUi,
  fillMainIssueTitle,
} from "./helpers/auth";

const SPRINTS_URL_PATTERN = /\/sprints$/;

test.describe("project sprints", () => {
  test("creates a sprint and assigns a ticket", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Sprint User",
      prefix: "sprints",
    });
    const issueTitle = `Sprint ticket ${crypto.randomUUID()}`;
    const sprintName = `Sprint ${crypto.randomUUID().slice(0, 8)}`;

    try {
      await fillMainIssueTitle(page, issueTitle);
      await page
        .getByLabel("Issue description")
        .fill("Created before assigning to a sprint.");
      await page.getByRole("button", { name: "Create work item" }).click();
      await expect(
        page.getByRole("heading", { name: issueTitle })
      ).toBeVisible();

      await page.getByRole("link", { exact: true, name: "Sprints" }).click();
      await expect(page).toHaveURL(SPRINTS_URL_PATTERN);

      await page.getByLabel("Sprint name").fill(sprintName);
      await page.getByLabel("Start date", { exact: true }).fill("2026-07-13");
      await page.getByLabel("End date", { exact: true }).fill("2026-07-27");
      await page
        .getByLabel("Sprint description")
        .fill("Two-week scope for the sprint test.");
      await page.getByRole("button", { name: "Create sprint" }).click();

      const sprintCard = page
        .locator("article")
        .filter({ hasText: sprintName });
      await expect(sprintCard).toBeVisible();
      await sprintCard
        .getByRole("button", { name: "Select a ticket…" })
        .click();
      await page.getByRole("option", { name: issueTitle }).click();

      await expect(sprintCard).toContainText(issueTitle);
      await expect(sprintCard).toContainText("0 / 1 tickets done");
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
