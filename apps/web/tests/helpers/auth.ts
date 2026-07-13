import { expect, type Page } from "@playwright/test";

export const DEFAULT_PASSWORD = "DevPassword123!";
export const WRONG_PASSWORD = "WrongPassword123!";
export const DASHBOARD_URL_PATTERN = /\/dashboard$/;
export const AUTHENTICATED_APP_URL_PATTERN =
  /\/(?:dashboard|workspace\/[^/]+(?:\/.*)?)$/;
export const LOGIN_URL_PATTERN = /\/login(?:\?.*)?$/;
export const LOGIN_WITHOUT_QUERY_URL_PATTERN = /\/login$/;
export const PROTECTED_REDIRECT_URL_PATTERN =
  /\/login\?returnTo=(?:%2F|\/)dashboard$/;
export const WORKSPACE_PROJECT_MODULE_URL_PATTERN =
  /\/workspace\/[^/]+\/projects\/[^/]+\/tickets$/;

export function uniqueEmail(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}@jet-black.local`;
}

export async function waitForHydration(page: Page) {
  await expect(page.locator("main[data-hydrated='true']")).toBeAttached();
  await expect(page.getByLabel("Email")).toBeVisible();
  await expect(page.getByLabel("Password")).toBeVisible();
}

export async function waitForDashboardReady(page: Page) {
  await expect(
    page.getByRole("button", { name: "Delete account" })
  ).toBeVisible({
    timeout: 15_000,
  });

  const issueTitle = page.getByRole("textbox", {
    exact: true,
    name: "Issue title",
  });
  const createDefaultWorkspace = page.getByRole("button", {
    name: "Create default workspace",
  });

  try {
    await expect(issueTitle).toBeVisible({ timeout: 10_000 });
    return;
  } catch {
    await expect(createDefaultWorkspace).toBeVisible();
    await createDefaultWorkspace.click({ force: true });
  }

  await expect(issueTitle).toBeVisible({
    timeout: 15_000,
  });
}

export async function fillMainIssueTitle(page: Page, title: string) {
  const issueTitle = page.getByRole("textbox", {
    exact: true,
    name: "Issue title",
  });

  await expect(issueTitle).toBeEditable();

  for (let attempt = 0; attempt < 3; attempt += 1) {
    await issueTitle.fill(title);

    if ((await issueTitle.inputValue()) === title) {
      return;
    }

    await page.waitForTimeout(100);
  }

  await expect(issueTitle).toHaveValue(title);
}

export async function openCreateAccountMode(page: Page) {
  await page.goto("/login");
  await waitForHydration(page);
  await page.getByRole("button", { name: "Create an account" }).first().click();
  await expect(page.getByLabel("Name")).toBeVisible();
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
  await expect(page).toHaveURL(AUTHENTICATED_APP_URL_PATTERN);
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

  if (AUTHENTICATED_APP_URL_PATTERN.test(page.url())) {
    await waitForDashboardReady(page);
    return;
  }

  await waitForHydration(page);
  await page.getByLabel("Email").fill(input.email);
  await page.getByLabel("Password").fill(input.password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page).toHaveURL(AUTHENTICATED_APP_URL_PATTERN);
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
  const signOutButton = page.getByRole("button", { name: "Sign out" });

  for (let attempt = 0; attempt < 3; attempt += 1) {
    await signOutButton.click({ force: true });

    try {
      await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN, {
        timeout: 5000,
      });
      return;
    } catch (error) {
      if (attempt === 2) {
        throw error;
      }
    }
  }
}

export async function deleteCurrentAccountWithUi(page: Page) {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    await page.getByRole("button", { name: "Delete account" }).click({
      force: true,
    });

    const confirmDelete = page.getByRole("button", {
      name: "Confirm delete account",
    });
    await expect(confirmDelete).toBeVisible();

    try {
      await confirmDelete.click({ force: true, timeout: 5000 });
      break;
    } catch (error) {
      if (attempt === 2) {
        throw error;
      }
    }
  }

  await expect(page).toHaveURL(LOGIN_URL_PATTERN, { timeout: 15_000 });
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

  await expect(deleteButton).toBeVisible();
  await deleteCurrentAccountWithUi(page);
}
