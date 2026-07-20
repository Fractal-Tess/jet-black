import { expect, test } from "@playwright/test";

import { cleanupAccountWithUi, createAccountWithUi } from "./helpers/auth";

const ANALYTICS_URL_PATTERN = /\/workspace\/[^/]+\/analytics$/;

test.describe("workspace analytics", () => {
  test("shows workspace metrics and project-filtered work item analytics", async ({
    page,
  }) => {
    const account = await createAccountWithUi(page, {
      name: "Analytics User",
      prefix: "analytics",
    });

    try {
      const analyticsLink = page.locator("aside").getByRole("link", {
        exact: true,
        name: "Analytics",
      });
      await expect(analyticsLink).toBeVisible();
      await analyticsLink.click();

      await expect(page).toHaveURL(ANALYTICS_URL_PATTERN);
      await expect(
        page.getByRole("heading", { level: 1, name: "Analytics" })
      ).toBeVisible();
      await expect(analyticsLink).toHaveAttribute("aria-current", "page");
      await expect(
        page.getByRole("button", { name: "Filter analytics by project" })
      ).toContainText("All projects");
      await expect(
        page.getByText("Total members", { exact: true })
      ).toBeVisible();
      await expect(
        page.getByText("Work items", { exact: true }).first()
      ).toBeVisible();
      await expect(
        page.getByRole("heading", { name: "Project progress" })
      ).toBeVisible();

      await page
        .getByRole("button", { name: "Filter analytics by project" })
        .click();
      await page.getByRole("option", { name: "Jet Black" }).click();
      await expect(
        page.getByRole("button", { name: "Filter analytics by project" })
      ).toContainText("Jet Black");

      await page.getByRole("tab", { name: "Work items" }).click();
      await expect(
        page.getByRole("heading", { name: "Created vs completed" })
      ).toBeVisible();
      await expect(
        page.getByRole("heading", { name: "Current work by status" })
      ).toBeVisible();

      await page.reload();
      await expect(page).toHaveURL(ANALYTICS_URL_PATTERN);
      await expect(
        page.getByRole("heading", { level: 1, name: "Analytics" })
      ).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
