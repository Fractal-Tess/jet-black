import { execFileSync } from "node:child_process";

/**
 * One-shot production migrator for the self-hosted Convex deployment.
 *
 * Docker Compose runs this after the Convex backend becomes healthy and before
 * starting the web app. It deploys the current schema and functions, configures
 * runtime environment variables, seeds initial data, and then exits.
 */

// These paths are provided by the pruned migrator image in convex/Dockerfile.
const CONVEX_CLI_PATH = "/app/node_modules/convex/bin/main.js";
const GENERATE_KEY_PATH = "/usr/local/bin/convex-generate-key";

// Validate all configuration before making any changes to the deployment.
const REQUIRED_ENVIRONMENT_VARIABLES = [
  "BETTER_AUTH_SECRET",
  "BETTER_AUTH_TRUSTED_ORIGINS",
  "CONVEX_INSTANCE_NAME",
  "CONVEX_INSTANCE_SECRET",
  "CONVEX_SELF_HOSTED_URL",
  "SITE_URL",
] as const;

type RequiredEnvironmentVariable =
  (typeof REQUIRED_ENVIRONMENT_VARIABLES)[number];

function requireEnvironment(name: RequiredEnvironmentVariable): string {
  const value = process.env[name]?.trim();

  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`);
  }

  return value;
}

const environment = {
  betterAuthSecret: requireEnvironment("BETTER_AUTH_SECRET"),
  betterAuthTrustedOrigins: requireEnvironment("BETTER_AUTH_TRUSTED_ORIGINS"),
  convexInstanceName: requireEnvironment("CONVEX_INSTANCE_NAME"),
  convexInstanceSecret: requireEnvironment("CONVEX_INSTANCE_SECRET"),
  convexSelfHostedUrl: requireEnvironment("CONVEX_SELF_HOSTED_URL"),
  siteUrl: requireEnvironment("SITE_URL"),
};

// The backend and migrator derive the same admin key from the shared instance
// name and secret, so the key itself never needs to be stored in configuration.
const adminKey = execFileSync(
  GENERATE_KEY_PATH,
  [environment.convexInstanceName, environment.convexInstanceSecret],
  { encoding: "utf8" }
).trim();

if (!adminKey) {
  throw new Error("Convex admin key generation returned an empty value.");
}

const processEnvironment = {
  ...process.env,
  CONVEX_SELF_HOSTED_ADMIN_KEY: adminKey,
  CONVEX_SELF_HOSTED_URL: environment.convexSelfHostedUrl,
};

// Run the Node-targeted Convex CLI synchronously so each step must succeed
// before the next starts. Output is forwarded to the container logs.
function runConvex(args: readonly string[]) {
  execFileSync(process.execPath, [CONVEX_CLI_PATH, ...args], {
    env: processEnvironment,
    stdio: "inherit",
  });
}

// Deploy application code and schema first.
runConvex(["deploy", "--typecheck", "disable"]);

// Configure values read by Convex functions and the Better Auth component.
runConvex(["env", "set", "SITE_URL", environment.siteUrl]);
runConvex(["env", "set", "BETTER_AUTH_SECRET", environment.betterAuthSecret]);
runConvex([
  "env",
  "set",
  "BETTER_AUTH_TRUSTED_ORIGINS",
  environment.betterAuthTrustedOrigins,
]);

// Seed required initial records. init:seed is idempotent and safe on restarts.
runConvex(["run", "init:seed", "{}"]);
