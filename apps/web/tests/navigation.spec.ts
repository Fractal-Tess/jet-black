import { expect, test } from "@playwright/test";

import {
  cleanupAccountWithUi,
  createAccountWithUi,
  WORKSPACE_PROJECT_MODULE_URL_PATTERN,
} from "./helpers/auth";

const INTAKE_URL_PATTERN = /\/intake$/;
const MODULES_URL_PATTERN = /\/modules$/;
const PAGES_URL_PATTERN = /\/pages$/;
const SPRINTS_URL_PATTERN = /\/sprints$/;

test.describe("workspace navigation", () => {
  test("uses URL-driven project modules", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Navigation User",
      prefix: "navigation",
    });

    try {
      const defaultProject = page.locator("aside").getByRole("link", {
        exact: true,
        name: "Jet Black",
      });
      await expect(defaultProject).toBeVisible();
      await defaultProject.click();

      await expect(page).toHaveURL(WORKSPACE_PROJECT_MODULE_URL_PATTERN);
      await expect(
        page.getByRole("link", { exact: true, name: "Tickets" })
      ).toBeVisible();
      await expect(
        page.getByRole("link", { exact: true, name: "Intake" })
      ).toBeVisible();
      await expect(
        page.getByRole("link", { exact: true, name: "Sprints" })
      ).toBeVisible();
      await expect(
        page.getByRole("link", { exact: true, name: "Modules" })
      ).toBeVisible();
      await expect(
        page.getByRole("link", { exact: true, name: "Pages" })
      ).toBeVisible();

      await page.getByRole("link", { exact: true, name: "Intake" }).click();
      await expect(page).toHaveURL(INTAKE_URL_PATTERN);
      await expect(
        page.getByRole("heading", { name: "New intake item" })
      ).toBeVisible();

      await page.getByRole("link", { exact: true, name: "Sprints" }).click();
      await expect(page).toHaveURL(SPRINTS_URL_PATTERN);
      await expect(
        page.getByRole("heading", { name: "New sprint" })
      ).toBeVisible();

      await page.getByRole("link", { exact: true, name: "Modules" }).click();
      await expect(page).toHaveURL(MODULES_URL_PATTERN);
      await expect(
        page.getByRole("heading", { name: "New module" })
      ).toBeVisible();

      await page.getByRole("link", { exact: true, name: "Pages" }).click();
      await expect(page).toHaveURL(PAGES_URL_PATTERN);
      await expect(
        page.getByRole("heading", { name: "New page" })
      ).toBeVisible();
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
