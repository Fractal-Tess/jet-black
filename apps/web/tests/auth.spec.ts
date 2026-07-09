import {
  type APIRequestContext,
  expect,
  type Page,
  test,
} from "@playwright/test";

const APP_ORIGIN = "http://127.0.0.1:5173";
const DEFAULT_PASSWORD = "DevPassword123!";
const DASHBOARD_URL_PATTERN = /\/dashboard$/;
const LOGIN_URL_PATTERN = /\/login(?:\?.*)?$/;
const LOGIN_WITHOUT_QUERY_URL_PATTERN = /\/login$/;
const PROTECTED_REDIRECT_URL_PATTERN = /\/login\?returnTo=(?:%2F|\/)dashboard$/;

function uniqueEmail(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}@jet-black.local`;
}

async function waitForHydration(page: Page) {
  await expect(page.locator("[data-hydrated='true']")).toBeAttached();
}

async function createAccount(request: APIRequestContext, name: string) {
  const email = uniqueEmail("auth");
  const response = await request.post("/api/auth/sign-up/email", {
    data: {
      email,
      name,
      password: DEFAULT_PASSWORD,
    },
    headers: {
      origin: APP_ORIGIN,
    },
  });

  expect(response.ok(), await response.text()).toBeTruthy();

  return { email, password: DEFAULT_PASSWORD };
}

async function signIn(page: Page, email: string, password: string) {
  await page.goto("/login");
  await waitForHydration(page);
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
}

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

  test("creates an account and persists its session after reload", async ({
    page,
  }) => {
    const email = uniqueEmail("signup");

    await page.goto("/login");
    await waitForHydration(page);
    await page.getByRole("button", { name: "Create an account" }).click();
    await page.getByLabel("Name").fill("Playwright User");
    await page.getByLabel("Email").fill(email);
    await page.getByLabel("Password").fill(DEFAULT_PASSWORD);
    await page.getByRole("button", { name: "Create account" }).click();

    await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
    await expect(
      page.getByRole("heading", { name: "Good to see you, Playwright" })
    ).toBeVisible();

    await page.reload();

    await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
    await expect(page.getByText(email)).toBeVisible();
  });

  test("logs in to an existing account", async ({ page, request }) => {
    const account = await createAccount(request, "Existing User");

    await signIn(page, account.email, account.password);

    await expect(
      page.getByRole("heading", { name: "Good to see you, Existing" })
    ).toBeVisible();
  });

  test("shows an error for invalid credentials", async ({ page }) => {
    await page.goto("/login");
    await waitForHydration(page);
    await page.getByLabel("Email").fill(uniqueEmail("missing"));
    await page.getByLabel("Password").fill("IncorrectPassword123!");
    await page.getByRole("button", { name: "Sign in", exact: true }).click();

    await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN);
    await expect(page.getByRole("alert")).toBeVisible();
  });

  test("signs out and invalidates access to protected routes", async ({
    page,
    request,
  }) => {
    const account = await createAccount(request, "Logout User");
    await signIn(page, account.email, account.password);

    await page.getByRole("button", { name: "Sign out" }).click();

    await expect(page).toHaveURL(LOGIN_WITHOUT_QUERY_URL_PATTERN);

    await page.goto("/dashboard");

    await expect(page).toHaveURL(PROTECTED_REDIRECT_URL_PATTERN);
  });

  test("redirects authenticated users away from login", async ({
    page,
    request,
  }) => {
    const account = await createAccount(request, "Redirect User");
    await signIn(page, account.email, account.password);

    await page.goto("/login");

    await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
  });

  test("returns to the originally requested protected route", async ({
    page,
    request,
  }) => {
    const account = await createAccount(request, "Return User");

    await page.goto("/dashboard");
    await expect(page).toHaveURL(LOGIN_URL_PATTERN);
    await waitForHydration(page);
    await page.getByLabel("Email").fill(account.email);
    await page.getByLabel("Password").fill(account.password);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();

    await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
  });

  test("does not redirect to an external return URL", async ({
    page,
    request,
  }) => {
    const account = await createAccount(request, "Safe Redirect User");

    await page.goto("/login?returnTo=https://example.com");
    await waitForHydration(page);
    await page.getByLabel("Email").fill(account.email);
    await page.getByLabel("Password").fill(account.password);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();

    await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
  });
});
