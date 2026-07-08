import type { APIRequestContext } from "@playwright/test";
import { expect, test } from "@playwright/test";

const LOGIN_URL_PATTERN = /\/login$/;
const DASHBOARD_URL_PATTERN = /\/dashboard$/;
const WELCOME_BACK_HEADING_PATTERN = /Welcome back/i;
const DEFAULT_PASSWORD = "DevPassword123!";
const AUTH_BACKEND_URL = "http://127.0.0.1:3211";
const APP_ORIGIN = "http://127.0.0.1:3000";

function uniqueEmail(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@jet-black.local`;
}

async function isAuthBackendAvailable(request: APIRequestContext) {
  try {
    const response = await request.get(`${AUTH_BACKEND_URL}/version`, {
      timeout: 2000,
    });
    return response.ok();
  } catch {
    return false;
  }
}

test("renders the landing page", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.getByText("A minimal starter for apps, auth, workers & scraping.")
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Dashboard", exact: true })
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "Login" })).toBeVisible();
});

test("renders the login page", async ({ page }) => {
  await page.goto("/login");

  await expect(page.getByText("Sign in to your account")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Sign in" }).first()
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Create account" })
  ).toBeVisible();
});

test("redirects signed-out users away from the dashboard", async ({ page }) => {
  await page.goto("/dashboard");

  await expect(page).toHaveURL(LOGIN_URL_PATTERN);
});

test("registers a new account from the login page", async ({
  page,
  request,
}) => {
  if (!(await isAuthBackendAvailable(request))) {
    test.info().annotations.push({
      type: "note",
      description: "Auth backend unavailable; register flow check skipped.",
    });
    return;
  }

  const email = uniqueEmail("signup");

  await page.goto("/login");
  await page.getByRole("button", { name: "Create account" }).click();
  await page.getByLabel("Name").fill("Playwright User");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(DEFAULT_PASSWORD);
  await page.locator("form button[type='submit']").click();

  await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
  await expect(
    page.getByRole("heading", { name: WELCOME_BACK_HEADING_PATTERN })
  ).toBeVisible();
});

test("logs in with an existing account", async ({ page, request }) => {
  if (!(await isAuthBackendAvailable(request))) {
    test.info().annotations.push({
      type: "note",
      description: "Auth backend unavailable; login flow check skipped.",
    });
    return;
  }

  const email = uniqueEmail("signin");

  const signUpResponse = await request.post("/api/auth/sign-up/email", {
    data: {
      email,
      name: "Sign In User",
      password: DEFAULT_PASSWORD,
    },
    headers: {
      origin: APP_ORIGIN,
    },
  });

  expect(signUpResponse.ok(), await signUpResponse.text()).toBeTruthy();

  await page.goto("/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(DEFAULT_PASSWORD);
  await page.locator("form button[type='submit']").click();

  await expect(page).toHaveURL(DASHBOARD_URL_PATTERN);
  await expect(
    page.getByRole("heading", { name: WELCOME_BACK_HEADING_PATTERN })
  ).toBeVisible();
});
