import { expect, test } from "@playwright/test";

import {
  cleanupAccountWithUi,
  createAccountWithUi,
  DASHBOARD_URL_PATTERN,
  DEFAULT_PASSWORD,
  deleteCurrentAccountWithUi,
  expectSignInFailure,
  LOGIN_URL_PATTERN,
  LOGIN_WITHOUT_QUERY_URL_PATTERN,
  PROTECTED_REDIRECT_URL_PATTERN,
  signInWithUi,
  signOutWithUi,
  uniqueEmail,
  WRONG_PASSWORD,
  waitForHydration,
} from "./helpers/auth";

test.describe("email and password authentication", () => {
  test("redirects the root route to login when signed out", async ({
    page,
  }) => {
    await page.goto("/");

    await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN);
    await waitForHydration(page);
    await expect(
      page.getByRole("heading", { name: "Work without the refresh button." })
    ).toBeVisible();
  });

  test("covers credential failures, session-gated work, logout, and account deletion", async ({
    page,
  }) => {
    const account = await createAccountWithUi(page, {
      name: "Lifecycle User",
      prefix: "auth-lifecycle",
    });
    const issueTitle = `Lifecycle issue ${crypto.randomUUID()}`;
    let deleted = false;

    try {
      await expect(
        page.getByRole("heading", { name: "Good to see you, Lifecycle" })
      ).toBeVisible();
      await expect(page.getByText(account.email)).toBeVisible();

      await page.reload();
      await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
      await expect(page.getByText(account.email)).toBeVisible();

      await signOutWithUi(page);

      await expectSignInFailure(page, {
        email: uniqueEmail("missing-correct-password"),
        password: DEFAULT_PASSWORD,
      });
      await expectSignInFailure(page, {
        email: account.email,
        password: WRONG_PASSWORD,
      });
      await expectSignInFailure(page, {
        email: uniqueEmail("missing-wrong-password"),
        password: WRONG_PASSWORD,
      });

      await signInWithUi(page, account);
      await page.getByLabel("Issue title").fill(issueTitle);
      await page
        .getByLabel("Issue description")
        .fill("Created by the authenticated lifecycle e2e flow.");
      await page.getByRole("button", { name: "Create issue" }).click();
      await expect(
        page.getByRole("heading", { name: issueTitle })
      ).toBeVisible();

      await signOutWithUi(page);
      await page.goto("/dashboard");
      await expect(page).toHaveURL(PROTECTED_REDIRECT_URL_PATTERN);

      await signInWithUi(page, account);
      await deleteCurrentAccountWithUi(page);
      deleted = true;

      await expectSignInFailure(page, account);
    } catch (error) {
      if (!deleted) {
        await cleanupAccountWithUi(page, account);
      }
      throw error;
    }
  });

  test("returns to the originally requested protected route", async ({
    page,
  }) => {
    const account = await createAccountWithUi(page, {
      name: "Return User",
      prefix: "auth-return",
    });

    try {
      await signOutWithUi(page);
      await page.goto("/dashboard");
      await expect(page).toHaveURL(LOGIN_URL_PATTERN);
      await waitForHydration(page);
      await page.getByLabel("Email").fill(account.email);
      await page.getByLabel("Password").fill(account.password);
      await page.getByRole("button", { name: "Sign in", exact: true }).click();

      await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });

  test("does not redirect to an external return URL", async ({ page }) => {
    const account = await createAccountWithUi(page, {
      name: "Safe Redirect User",
      prefix: "auth-safe-redirect",
    });

    try {
      await signOutWithUi(page);
      await page.goto("/login?returnTo=https://example.com");
      await waitForHydration(page);
      await page.getByLabel("Email").fill(account.email);
      await page.getByLabel("Password").fill(account.password);
      await page.getByRole("button", { name: "Sign in", exact: true }).click();

      await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
    } finally {
      await cleanupAccountWithUi(page, account);
    }
  });
});
