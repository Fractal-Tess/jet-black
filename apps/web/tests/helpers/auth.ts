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
  const sidebarReady = page
    .locator("aside")
    .getByRole("link", { exact: true, name: "Home" });
  await expect(sidebarReady).toBeVisible({ timeout: 15_000 });

  const dashboardReady = page.getByRole("button", { name: "Add work item" });

  // Fast path: workspace dashboard already loaded
  await expect(dashboardReady)
    .toBeVisible({ timeout: 5000 })
    .catch(() => {
      // No workspace yet — proceed to onboarding or legacy flow
    });
  if (await dashboardReady.isVisible()) {
    return;
  }

  // Onboarding path: "Set up workspace" link for new accounts
  const setUpWorkspace = page.getByRole("link", { name: "Set up workspace" });
  try {
    await expect(setUpWorkspace).toBeVisible({ timeout: 5000 });
  } catch {
    // Legacy fallback: "Create default workspace" button
    const createDefaultWorkspace = page.getByRole("button", {
      name: "Create default workspace",
    });
    await expect(createDefaultWorkspace).toBeVisible();
    await createDefaultWorkspace.click({ force: true });
    await expect(dashboardReady).toBeVisible({ timeout: 15_000 });
    return;
  }

  // --- Onboarding flow: Set up workspace → 2-step onboarding → project creation ---

  await setUpWorkspace.click();

  // Step 1: Profile setup
  await expect(
    page.getByRole("heading", { name: "Welcome to Jet Black" })
  ).toBeVisible({ timeout: 10_000 });
  await page.getByRole("button", { name: "Continue" }).click();

  // Step 2: Create workspace with a unique name
  await expect(
    page.getByRole("heading", { name: "Create your workspace" })
  ).toBeVisible({ timeout: 10_000 });
  const workspaceName = `ws-${crypto.randomUUID().slice(0, 8)}`;
  await page.getByLabel("Workspace name").fill(workspaceName);
  await page.getByRole("button", { name: "Create workspace" }).click();

  // Step 3: Create a default project via the sidebar modal.
  // The onboarding workspace mutation does not create a project, so the
  // workspace page shows "Create your first project" next to the sidebar.
  await expect(
    page.locator("aside").getByRole("button", { name: "Create project" })
  ).toBeVisible({ timeout: 15_000 });
  await page
    .locator("aside")
    .getByRole("button", { name: "Create project" })
    .click();
  await expect(page.getByLabel("Project name")).toBeVisible({
    timeout: 10_000,
  });
  await page.getByLabel("Project name").fill("Jet Black");
  await page.getByTestId("create-project-submit").click();

  // Wait for tickets page with the kanban board
  await expect(dashboardReady).toBeVisible({ timeout: 20_000 });

  // Navigate back to /dashboard so callers get the expected URL
  await page.goto("/dashboard");
  await expect(dashboardReady).toBeVisible({ timeout: 15_000 });
}

export async function fillMainIssueTitle(page: Page, title: string) {
  // Open the issue creation modal via the header button
  await page.getByRole("button", { name: "Add work item" }).click();

  // The modal shows a "Title" input; wait for it to be editable
  const issueTitle = page.getByPlaceholder("What needs to be done?");
  await expect(issueTitle).toBeEditable({ timeout: 10_000 });

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
  const userMenuTrigger = page.getByRole("button", { name: "User menu" });
  const signOutButton = page.getByRole("button", { name: "Sign out" });

  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      if (LOGIN_WITHOUT_QUERY_URL_PATTERN.test(page.url())) {
        return;
      }

      if (!(await signOutButton.isVisible())) {
        await userMenuTrigger.click();
        await expect(signOutButton).toBeVisible({ timeout: 5000 });
      }

      await signOutButton.click({ force: true });
      await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN, {
        timeout: 10_000,
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
  const deleteAccountBtn = page.getByRole("button", {
    name: "Delete account",
    exact: true,
  });

  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      if (!(await deleteAccountBtn.isVisible())) {
        await page.goto("/settings/profile/danger");
        await expect(
          page.getByRole("heading", { name: "Danger zone" })
        ).toBeVisible({ timeout: 10_000 });
      }

      await expect(deleteAccountBtn).toBeVisible({ timeout: 5000 });
      await deleteAccountBtn.click();

      const confirmDelete = page.getByRole("button", {
        name: "Confirm delete account",
        exact: true,
      });
      await expect(confirmDelete).toBeVisible({ timeout: 5000 });
      await confirmDelete.click();
      await expect(page).toHaveURL(LOGIN_URL_PATTERN, { timeout: 15_000 });
      return;
    } catch (error) {
      if (attempt === 2) {
        throw error;
      }
    }
  }
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

  const deleteButton = page.getByRole("button", {
    name: "Delete account",
    exact: true,
  });
  if (!(await deleteButton.isVisible())) {
    await signInWithUi(page, account);
    await page.goto("/settings/profile/danger");
    await expect(
      page.getByRole("heading", { name: "Danger zone" })
    ).toBeVisible({ timeout: 10_000 });
  }

  await expect(deleteButton).toBeVisible();
  await deleteCurrentAccountWithUi(page);
}
