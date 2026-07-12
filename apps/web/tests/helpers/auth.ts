import { expect, type Page } from "@playwright/test";

export const DEFAULT_PASSWORD = "DevPassword123!";
export const WRONG_PASSWORD = "WrongPassword123!";
export const DASHBOARD_URL_PATTERN = /\/dashboard$/;
export const LOGIN_URL_PATTERN = /\/login(?:\?.*)?$/;
export const LOGIN_WITHOUT_QUERY_URL_PATTERN = /\/login$/;
export const PROTECTED_REDIRECT_URL_PATTERN =
  /\/login\?returnTo=(?:%2F|\/)dashboard$/;

export function uniqueEmail(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}@jet-black.local`;
}

export async function waitForHydration(page: Page) {
  await expect(page.locator("[data-hydrated='true']")).toBeAttached();
}

export async function waitForDashboardReady(page: Page) {
  await expect(
    page.getByRole("button", { name: "Delete account" })
  ).toBeVisible({
    timeout: 15_000,
  });
  await expect(page.getByLabel("Issue title")).toBeVisible({
    timeout: 15_000,
  });
}

export async function openCreateAccountMode(page: Page) {
  await page.goto("/login");
  await waitForHydration(page);
  await page.getByRole("button", { name: "Create an account" }).first().click();
}

export async function createAccountWithUi(
  page: Page,
  input: {
    email?: string;
    name: string;
    password?: string;
    prefix?: string;
  }
) {
  const email = input.email ?? uniqueEmail(input.prefix ?? "auth");
  const password = input.password ?? DEFAULT_PASSWORD;

  await openCreateAccountMode(page);
  await page.getByLabel("Name").fill(input.name);
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
  await waitForDashboardReady(page);

  return { email, password };
}

export async function signInWithUi(
  page: Page,
  input: {
    email: string;
    password: string;
  }
) {
  await page.goto("/login");

  if (DASHBOARD_URL_PATTERN.test(page.url())) {
    await waitForDashboardReady(page);
    return;
  }

  await waitForHydration(page);
  await page.getByLabel("Email").fill(input.email);
  await page.getByLabel("Password").fill(input.password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
  await waitForDashboardReady(page);
}

export async function expectSignInFailure(
  page: Page,
  input: {
    email: string;
    password: string;
  }
) {
  await page.goto("/login");
  await waitForHydration(page);
  await page.getByLabel("Email").fill(input.email);
  await page.getByLabel("Password").fill(input.password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();

  await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN);
  await expect(page.getByRole("alert")).toBeVisible();
}

export async function signOutWithUi(page: Page) {
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN);
}

export async function deleteCurrentAccountWithUi(page: Page) {
  await page.getByRole("button", { name: "Delete account" }).click();
  await page.getByRole("button", { name: "Confirm delete account" }).click();
  await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN);
}

export async function cleanupAccountWithUi(
  page: Page,
  account: {
    email: string;
    password: string;
  } | null
) {
  if (!account) {
    return;
  }

  const deleteButton = page.getByRole("button", { name: "Delete account" });
  if (!(await deleteButton.isVisible())) {
    await signInWithUi(page, account);
  }

  await deleteCurrentAccountWithUi(page);
}
