import { expect, test } from "@playwright/test";

import {
  cleanupAccountWithUi,
  createAccountWithUi,
  fillMainIssueTitle,
  signInWithUi,
} from "./helpers/auth";

const REORDER_UP_LABEL_PATTERN = /Reorder .* up/;
const PRIORITY_HIGH_LABEL_PATTERN = /Priority High/;
const INTAKE_URL_PATTERN = /\/intake$/;

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
    const attachmentName = `Spec ${crypto.randomUUID().slice(0, 8)}`;
    const attachmentUrl = "https://example.com/spec";
    const subIssueTitle = `Sub issue ${crypto.randomUUID()}`;

    try {
      await fillMainIssueTitle(page, title);
      await page
        .getByLabel("Issue description")
        .fill("This issue was created by Playwright.");
      await page.getByLabel("Priority").selectOption("high");
      await page.getByRole("button", { name: "Create work item" }).click();

      await expect(page.getByRole("heading", { name: title })).toBeVisible();
      const issueCard = page.locator("article").filter({ hasText: title });
      await expect(issueCard).toBeVisible();
      await expect(
        issueCard.getByLabel(PRIORITY_HIGH_LABEL_PATTERN)
      ).toBeVisible();
      await issueCard.getByLabel("Move").click();
      await page.getByRole("menuitem", { name: "Done" }).click();
      await expect(page.getByLabel("Done column")).toContainText(title);

      await page.getByRole("button", { name: "Edit" }).click();
      await page.getByLabel("Title", { exact: true }).fill(updatedTitle);
      await page.getByLabel("Estimate", { exact: true }).fill("3");
      await page.getByLabel("Start date", { exact: true }).fill("2026-07-14");
      await page.locator("[data-testid='issue-state-select']").selectOption({
        label: "In progress",
      });
      await page.getByLabel("Target date", { exact: true }).fill("2026-07-21");
      await page.getByRole("button", { name: "Save changes" }).click();

      await expect(
        page.getByRole("heading", { name: updatedTitle })
      ).toBeVisible();
      await expect(page.getByText("3 points")).toBeVisible();
      await expect(page.getByText("2026-07-14")).toBeVisible();
      await expect(page.getByText("2026-07-21")).toBeVisible();
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

      await page.getByLabel("Attachment name").fill(attachmentName);
      await page.getByLabel("Attachment URL").fill(attachmentUrl);
      await page.getByRole("button", { name: "Add attachment" }).click();
      await expect(
        page.getByRole("link", { name: attachmentName })
      ).toBeVisible();

      await page.getByLabel("Sub-issue title").fill(subIssueTitle);
      await page.getByRole("button", { exact: true, name: "Add" }).click();
      await expect(
        page.locator("article").filter({ hasText: subIssueTitle })
      ).toHaveCount(2);

      await page.getByLabel("New comment").fill(comment);
      await page.getByRole("button", { exact: true, name: "Comment" }).click();

      await expect(page.getByText(comment)).toBeVisible();
      await expect(
        page
          .locator("article")
          .filter({ hasText: updatedTitle })
          .getByLabel("1 comment")
      ).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("reorders issues inside a kanban state", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Order User",
      prefix: "issues-order",
    });
    const firstTitle = "Wire realtime issue updates";
    const secondTitle = `Ordered issue ${crypto.randomUUID()}`;

    try {
      const todoColumn = page.getByLabel("Todo column");
      await expect(todoColumn).toContainText(firstTitle);

      await page.getByLabel("Quick issue title for Todo").fill(secondTitle);
      await page.getByRole("button", { name: "Add issue to Todo" }).click();
      await expect(page.getByText(secondTitle).first()).toBeVisible();

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

  test("creates an issue directly inside a kanban state", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Quick User",
      prefix: "issues-quick",
    });
    const title = `Quick issue ${crypto.randomUUID()}`;

    try {
      await page.getByLabel("Quick issue title for Done").fill(title);
      await page.getByRole("button", { name: "Add issue to Done" }).click();

      await expect(page.getByLabel("Done column")).toContainText(title);
      await expect(page.getByRole("heading", { name: title })).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("searches and archives issues from the board", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Archive User",
      prefix: "issues-archive",
    });
    const archiveTitle = `Archive issue ${crypto.randomUUID()}`;
    const seededTitle = "Wire realtime issue updates";

    try {
      const todoColumn = page.getByLabel("Todo column");
      const archiveCard = todoColumn.locator("article").filter({
        hasText: archiveTitle,
      });
      const seededCard = todoColumn.locator("article").filter({
        hasText: seededTitle,
      });

      await page.getByLabel("Quick issue title for Todo").fill(archiveTitle);
      await page.getByRole("button", { name: "Add issue to Todo" }).click();

      await expect(archiveCard).toBeVisible({ timeout: 10_000 });

      await page.getByLabel("Search issues").fill(archiveTitle);
      await expect(archiveCard).toBeVisible({ timeout: 10_000 });
      await expect(seededCard).toBeHidden({ timeout: 10_000 });

      await archiveCard
        .getByRole("button")
        .first()
        .click({ force: true, timeout: 10_000 });
      await expect(
        page.getByRole("heading", { name: archiveTitle })
      ).toBeVisible({ timeout: 10_000 });
      await page
        .getByRole("button", { exact: true, name: "Archive" })
        .click({ force: true, timeout: 10_000 });

      await expect(archiveCard).toBeHidden({ timeout: 10_000 });
      await page.getByLabel("Search issues").fill("");
      await expect(seededCard).toBeVisible({ timeout: 10_000 });
      await expect(archiveCard).toBeHidden({ timeout: 10_000 });
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("groups issues by state in list view", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "List User",
      prefix: "issues-list",
    });
    const doneTitle = "Sketch pull request review workflow";
    const todoTitle = "Wire realtime issue updates";
    const quickTitle = `Listed issue ${crypto.randomUUID()}`;

    try {
      await page.getByRole("button", { exact: true, name: "List" }).click();

      const todoGroup = page.getByLabel("Todo group");
      const doneGroup = page.getByLabel("Done group");

      await expect(todoGroup).toContainText(todoTitle);
      await expect(doneGroup).toContainText(doneTitle);
      await expect(todoGroup).not.toContainText(doneTitle);

      await page.getByLabel("Toggle Todo group").click();
      await expect(todoGroup).not.toContainText(todoTitle);
      await page.getByLabel("Toggle Todo group").click();
      await expect(todoGroup).toContainText(todoTitle);

      await todoGroup.getByLabel("Quick issue title for Todo").fill(quickTitle);
      await todoGroup.getByLabel("Add issue to Todo").click();
      await expect(todoGroup).toContainText(quickTitle);
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("accepts intake items into issues", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Intake User",
      prefix: "issues-intake",
    });
    const title = `Intake request ${crypto.randomUUID()}`;

    try {
      await page.getByRole("link", { exact: true, name: "Intake" }).click();
      await expect(page).toHaveURL(INTAKE_URL_PATTERN);

      await page.getByLabel("Intake title").fill(title);
      await page.getByLabel("Intake source").fill("support");
      await page
        .getByLabel("Intake description")
        .fill("Created by the intake e2e flow.");
      await page.getByRole("button", { name: "Add intake item" }).click();

      await expect(page.getByText(title)).toBeVisible();
      await page.getByRole("button", { exact: true, name: "Accept" }).click();

      await expect(page.getByRole("heading", { name: title })).toBeVisible();
      await expect(page.getByLabel("Todo column")).toContainText(title);
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("moves an issue between states with drag and drop", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Drag User",
      prefix: "issues-drag",
    });
    const title = `Dragged issue ${crypto.randomUUID()}`;

    try {
      await page.getByLabel("Quick issue title for Todo").fill(title);
      await page.getByRole("button", { name: "Add issue to Todo" }).click();

      const issueCard = page.locator("article").filter({ hasText: title });
      await expect(page.getByLabel("Todo column")).toContainText(title);
      const dataTransfer = await page.evaluateHandle(() => new DataTransfer());
      await issueCard.dispatchEvent("dragstart", { dataTransfer });
      await page.getByLabel("Done column").dispatchEvent("dragover", {
        dataTransfer,
      });
      await page.getByLabel("Done column").dispatchEvent("drop", {
        dataTransfer,
      });
      await issueCard.dispatchEvent("dragend", { dataTransfer });

      await expect(page.getByLabel("Done column")).toContainText(title);
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("syncs issue state changes across browser contexts", async ({
    browser,
    page,
  }) => {
    const account = await createAccountWithUi(page, {
      name: "Realtime User",
      prefix: "issues-realtime",
    });
    const secondPage = await browser.newPage();
    const title = `Realtime synced issue ${crypto.randomUUID()}`;

    try {
      await signInWithUi(secondPage, account);

      await page.getByLabel("Quick issue title for Todo").fill(title);
      await page.getByRole("button", { name: "Add issue to Todo" }).click();

      await expect(page.getByLabel("Todo column")).toContainText(title);
      await expect(secondPage.getByLabel("Todo column")).toContainText(title);

      await page
        .locator("article")
        .filter({ hasText: title })
        .getByLabel("Move")
        .click();
      await page.getByRole("menuitem", { name: "Done" }).click();

      await expect(secondPage.getByLabel("Done column")).toContainText(title);
    } finally {
      await secondPage.close();
      await cleanupAccountWithUi(page, account);
    }
  });

  test("creates a project and opens it", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Project User",
      prefix: "issues-project",
    });
    const projectName = `Ops ${crypto.randomUUID().slice(0, 8)}`;
    const projectKey = `P${crypto.randomUUID().replace(/-/g, "").slice(0, 5)}`;
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
      await expect(page.getByText("0 tickets")).toBeVisible();
      await expect(
        page.getByText(`${projectKey.toUpperCase()}-1`)
      ).toBeHidden();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
