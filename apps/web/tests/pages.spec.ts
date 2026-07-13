import { expect, test } from "@playwright/test";

import { cleanupAccountWithUi, createAccountWithUi } from "./helpers/auth";

const PAGES_URL_PATTERN = /\/pages$/;

test.describe("project pages", () => {
  test("creates and edits a project doc", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Pages User",
      prefix: "pages",
    });
    const pageTitle = `Spec ${crypto.randomUUID().slice(0, 8)}`;
    const updatedTitle = `${pageTitle} updated`;
    const pageContent = `Initial doc body ${crypto.randomUUID()}`;
    const updatedContent = `Updated doc body ${crypto.randomUUID()}`;

    try {
      await page.getByRole("link", { exact: true, name: "Pages" }).click();
      await expect(page).toHaveURL(PAGES_URL_PATTERN);

      await page.getByLabel("Page title").fill(pageTitle);
      await page.getByLabel("Page icon").fill("✦");
      await page.getByLabel("Page content").fill(pageContent);
      await page.getByRole("button", { name: "Create page" }).click();

      await expect(page.getByRole("button", { name: pageTitle })).toBeVisible();
      await expect(
        page.getByRole("heading", { name: pageTitle })
      ).toBeVisible();
      await expect(page.getByText(pageContent)).toBeVisible();

      await page.getByLabel("Title", { exact: true }).fill(updatedTitle);
      await page.getByLabel("Content", { exact: true }).fill(updatedContent);
      await page.getByRole("button", { name: "Save page" }).click();

      await expect(
        page.getByRole("heading", { name: updatedTitle })
      ).toBeVisible();
      await expect(page.getByText(updatedContent)).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
