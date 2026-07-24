import { expect, test } from "@playwright/test";

const EMAIL = "dev@jet-black.local";
const PASSWORD = "jet-black-development";
const ANALYTICS_PATH = /\/analytics$/;
const SETTINGS_PATH = /\/settings$/;

async function signIn(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("Email").fill(EMAIL);
  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Enter workspace" }).click();
  await expect(page.getByRole("heading", { name: "Tickets" })).toBeVisible();
}

test("authenticates, creates a ticket, and preserves deep links", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Password").fill("incorrect-password");
  await page.getByRole("button", { name: "Enter workspace" }).click();
  await expect(page.getByRole("alert")).toContainText("Invalid credentials");

  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Enter workspace" }).click();
  await expect(page.getByRole("heading", { name: "Tickets" })).toBeVisible();

  const title = `Rust transport ${crypto.randomUUID()}`;
  await page.getByRole("button", { name: "New ticket" }).click();
  await page.getByLabel("Title").fill(title);
  await page
    .getByLabel("Description")
    .fill("Created through the static Jet Black client.");
  await page.getByLabel("Priority").selectOption("high");
  await page.getByRole("button", { name: "Create ticket" }).click();
  await expect(page.getByRole("heading", { name: title })).toBeVisible();

  await page.getByRole("button", { name: "Analytics" }).click();
  await expect(page).toHaveURL(ANALYTICS_PATH);
  await expect(page.getByRole("heading", { name: "Analytics" })).toBeVisible();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Analytics" })).toBeVisible();
});

test("supports the mobile navigation and list view", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page);
  await page.getByRole("button", { name: "Open navigation" }).click();
  await expect(page.getByRole("navigation")).toBeVisible();
  await page.getByRole("button", { name: "Tickets", exact: false }).click();
  await page.getByRole("button", { name: "List view" }).click();
  await expect(page.getByText("Ticket", { exact: true })).toBeVisible();
});

test("can point the static client at another instance", async ({ page }) => {
  await signIn(page);
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByLabel("Instance URL").fill("");
  await page.getByRole("button", { name: "Save and reconnect" }).click();
  await expect(page).toHaveURL(SETTINGS_PATH);
  await expect(page.getByLabel("Instance URL")).toHaveValue("");
});
