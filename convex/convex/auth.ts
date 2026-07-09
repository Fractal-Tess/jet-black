import { createClient, type GenericCtx } from "@convex-dev/better-auth";
import { convex } from "@convex-dev/better-auth/plugins";
import type { BetterAuthOptions } from "better-auth";
import { betterAuth } from "better-auth/minimal";
import { ConvexError } from "convex/values";

import { components } from "./_generated/api";
import type { DataModel } from "./_generated/dataModel";
import { query } from "./_generated/server";
import authConfig from "./auth.config";
import authSchema from "./betterAuth/schema";

const DEFAULT_LOCAL_AUTH_SECRET =
  "7VqVpUDmlFBw6VgsQIbEcXfq8Zli81fLL/FzYEa2LtI=";
const DEFAULT_LOCAL_SITE_URL = "http://127.0.0.1:3000";
const DEFAULT_TRUSTED_ORIGINS = [
  "http://127.0.0.1:3000",
  "http://127.0.0.1:3100",
  "http://localhost:3000",
  "http://localhost:3100",
  "http://vd.netbird.cloud:3000",
  "https://vd.netbird.cloud",
] as const;

function getBaseUrl() {
  return process.env.SITE_URL ?? DEFAULT_LOCAL_SITE_URL;
}

function getAllowedHosts() {
  const fallbackUrl = getBaseUrl();
  const hosts = new Set<string>();
  const candidates = [fallbackUrl, ...getTrustedOrigins()];

  for (const candidate of candidates) {
    if (!candidate) {
      continue;
    }

    try {
      const url = new URL(candidate);
      hosts.add(url.hostname);
      hosts.add(url.host);
    } catch {
      hosts.add(candidate);
    }
  }

  return [...hosts];
}

function getBetterAuthSecret() {
  return process.env.BETTER_AUTH_SECRET ?? DEFAULT_LOCAL_AUTH_SECRET;
}

function getTrustedOrigins() {
  return [
    ...DEFAULT_TRUSTED_ORIGINS,
    ...(process.env.BETTER_AUTH_TRUSTED_ORIGINS ?? "")
      .split(",")
      .map((origin) => origin.trim())
      .filter(Boolean),
  ];
}

export const authComponent = createClient<DataModel, typeof authSchema>(
  components.betterAuth,
  {
    local: {
      schema: authSchema,
    },
  }
);

export const createAuthOptions = (ctx: GenericCtx<DataModel>) =>
  ({
    baseURL: {
      allowedHosts: getAllowedHosts(),
      fallback: getBaseUrl(),
      protocol: "http",
    },
    secret: getBetterAuthSecret(),
    trustedOrigins: getTrustedOrigins(),
    database: authComponent.adapter(ctx),
    emailAndPassword: {
      enabled: true,
      requireEmailVerification: false,
    },
    plugins: [convex({ authConfig })],
  }) satisfies BetterAuthOptions;

export const createAuth = (ctx: GenericCtx<DataModel>) =>
  betterAuth(createAuthOptions(ctx));

export const getCurrentUser = query({
  args: {},
  handler: async (ctx) => {
    try {
      return await authComponent.getAuthUser(ctx);
    } catch (error) {
      if (
        error instanceof ConvexError &&
        typeof error.data === "string" &&
        error.data === "Unauthenticated"
      ) {
        return null;
      }

      throw error;
    }
  },
});
