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
  await page.getByRole("button", { name: title, exact: false }).click();
  await page.getByLabel("Workflow state").selectOption("started");
  await expect(
    page
      .locator(".board-column")
      .filter({ hasText: "IN PROGRESS" })
      .getByRole("heading", { name: title })
  ).toBeVisible();

  await page
    .getByRole("complementary", { name: "Ticket details" })
    .getByRole("button", { name: "Close ticket details" })
    .click();
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
  const instanceOrigin = new URL(page.url()).origin;
  await page.getByLabel("Instance URL").fill(instanceOrigin);
  await page.getByRole("button", { name: "Save and reconnect" }).click();
  await expect(page).toHaveURL(SETTINGS_PATH);
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await page.getByLabel("Instance URL").fill("");
  await page.getByRole("button", { name: "Save and reconnect" }).click();
  await expect(page).toHaveURL(SETTINGS_PATH);
  await expect(page.getByLabel("Instance URL")).toHaveValue("");
});

test("creates durable intake, sprint, module, and page records", async ({
  page,
}) => {
  await signIn(page);
  const suffix = crypto.randomUUID().slice(0, 8);

  await page.getByRole("button", { name: "Intake", exact: true }).click();
  await page.getByRole("button", { name: "New intake item" }).click();
  await page.getByLabel("Name").fill(`Request ${suffix}`);
  await page.getByLabel("Description").fill("Incoming product request");
  await page.getByLabel("Submitter email").fill("requester@example.com");
  await page
    .locator(".modal-card")
    .getByRole("button", { exact: true, name: "Create intake" })
    .click();
  await expect(
    page.getByRole("heading", { name: `Request ${suffix}` })
  ).toBeVisible();

  await page.getByRole("button", { name: "Sprints", exact: true }).click();
  await page.getByRole("button", { name: "New sprint" }).click();
  await page.getByLabel("Name").fill(`Sprint ${suffix}`);
  await page.getByLabel("Starts").fill("2026-07-27");
  await page.getByLabel("Ends").fill("2026-08-07");
  await page
    .locator(".modal-card")
    .getByRole("button", { exact: true, name: "Create sprint" })
    .click();
  await expect(
    page.getByRole("heading", { name: `Sprint ${suffix}` })
  ).toBeVisible();

  await page.getByRole("button", { name: "Modules", exact: true }).click();
  await page.getByRole("button", { name: "New module" }).click();
  await page.getByLabel("Name").fill(`Module ${suffix}`);
  await page.getByLabel("Target date").fill("2026-08-31");
  await page
    .locator(".modal-card")
    .getByRole("button", { exact: true, name: "Create module" })
    .click();
  await expect(
    page.getByRole("heading", { name: `Module ${suffix}` })
  ).toBeVisible();

  await page.getByRole("button", { name: "Pages", exact: true }).click();
  await page.getByRole("button", { name: "New page" }).click();
  await page.getByLabel("Title").fill(`Architecture ${suffix}`);
  await page.getByLabel("Content").fill("Durable control-plane context");
  await page
    .locator(".modal-card")
    .getByRole("button", { exact: true, name: "Create page" })
    .click();
  await expect(
    page.getByRole("heading", { name: `Architecture ${suffix}` })
  ).toBeVisible();
});
