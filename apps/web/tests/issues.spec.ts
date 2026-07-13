import { expect, test } from "@playwright/test";

import {
  cleanupAccountWithUi,
  createAccountWithUi,
  DASHBOARD_URL_PATTERN,
} from "./helpers/auth";

const REORDER_UP_LABEL_PATTERN = /Reorder .* up/;

test.describe("issue workspace", () => {
  test("creates, updates, and comments on an issue", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Issue User",
      prefix: "issues",
    });
    const title = `Realtime issue ${crypto.randomUUID()}`;
    const updatedTitle = `${title} updated`;
    const comment = `Looks good ${crypto.randomUUID()}`;
    const label = `Frontend ${crypto.randomUUID().slice(0, 8)}`;

    try {
      await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
      await expect(
        page.getByRole("heading", { name: "Good to see you, Issue" })
      ).toBeVisible();
      await page.getByLabel("Issue title").fill(title);
      await page
        .getByLabel("Issue description")
        .fill("This issue was created by Playwright.");
      await page.getByLabel("Priority").selectOption("high");
      await page.getByRole("button", { name: "Create issue" }).click();

      await expect(page.getByRole("heading", { name: title })).toBeVisible();
      const issueCard = page.locator("article").filter({ hasText: title });
      await expect(issueCard).toBeVisible();
      await expect(issueCard.getByText("High")).toBeVisible();
      await issueCard.getByLabel("Move").selectOption({ label: "Done" });
      await expect(page.getByLabel("Done column")).toContainText(title);

      await page.getByRole("button", { name: "Edit" }).click();
      await page.getByLabel("Title", { exact: true }).fill(updatedTitle);
      await page.locator("[data-testid='issue-state-select']").selectOption({
        label: "In progress",
      });
      await page.getByRole("button", { name: "Save changes" }).click();

      await expect(
        page.getByRole("heading", { name: updatedTitle })
      ).toBeVisible();
      await expect(page.getByLabel("In progress column")).toContainText(
        updatedTitle
      );

      await page.getByLabel("New label name").fill(label);
      await page.getByRole("button", { name: "Create label" }).click();
      await expect(page.getByLabel(`Toggle ${label} label`)).toBeVisible();
      await page.getByLabel(`Toggle ${label} label`).check();
      await expect(
        page
          .locator("article")
          .filter({ hasText: updatedTitle })
          .getByText(label)
      ).toBeVisible();

      await page.getByLabel("New comment").fill(comment);
      await page.getByRole("button", { exact: true, name: "Comment" }).click();

      await expect(page.getByText(comment)).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("reorders issues inside a kanban state", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Order User",
      prefix: "issues-order",
    });
    const firstTitle = `Ordered issue A ${crypto.randomUUID()}`;
    const secondTitle = `Ordered issue B ${crypto.randomUUID()}`;

    try {
      await page.getByLabel("Issue title").fill(firstTitle);
      await page.getByRole("button", { name: "Create issue" }).click();
      await expect(page.getByText(firstTitle).first()).toBeVisible();

      await page.getByLabel("Issue title").fill(secondTitle);
      await page.getByRole("button", { name: "Create issue" }).click();
      await expect(page.getByText(secondTitle).first()).toBeVisible();

      const todoColumn = page.getByLabel("Todo column");
      await expect(todoColumn).toContainText(firstTitle);
      await expect(todoColumn).toContainText(secondTitle);

      const initialColumnText = (await todoColumn.textContent()) ?? "";
      expect(initialColumnText.indexOf(firstTitle)).toBeLessThan(
        initialColumnText.indexOf(secondTitle)
      );

      await page
        .locator("article")
        .filter({ hasText: secondTitle })
        .getByLabel(REORDER_UP_LABEL_PATTERN)
        .click();

      await expect
        .poll(async () => {
          const columnText = (await todoColumn.textContent()) ?? "";

          return (
            columnText.indexOf(secondTitle) >= 0 &&
            columnText.indexOf(secondTitle) < columnText.indexOf(firstTitle)
          );
        })
        .toBe(true);
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("creates a project and creates an issue inside it", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Project User",
      prefix: "issues-project",
    });
    const projectName = `Ops ${crypto.randomUUID().slice(0, 8)}`;
    const projectKey = `P${crypto.randomUUID().replace(/-/g, "").slice(0, 5)}`;
    const issueTitle = `Project scoped issue ${crypto.randomUUID()}`;

    try {
      await page.getByRole("button", { name: "Create project" }).click();
      await page.getByLabel("Project name").fill(projectName);
      await page.getByLabel("Project key").fill(projectKey);
      await page.locator("[data-testid='create-project-submit']").click();

      const projectLink = page.getByRole("link", {
        exact: true,
        name: projectName,
      });
      await expect(projectLink).toBeVisible({ timeout: 15_000 });
      await projectLink.click();
      await expect(
        page.getByRole("heading", { name: projectName }).first()
      ).toBeVisible();
      await expect(page.getByText("0 issues")).toBeVisible();

      const issueTitleInput = page.getByRole("textbox", {
        name: "Issue title",
      });
      await expect(issueTitleInput).toBeEditable();
      await issueTitleInput.fill(issueTitle);
      await expect(issueTitleInput).toHaveValue(issueTitle);
      const createIssueButton = page.getByRole("button", {
        name: "Create issue",
      });
      await expect(createIssueButton).toBeEnabled();
      await createIssueButton.click();

      await expect(
        page.getByRole("heading", { name: issueTitle })
      ).toBeVisible();
      await expect(
        page.getByText(`${projectKey.toUpperCase()}-1`).first()
      ).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
