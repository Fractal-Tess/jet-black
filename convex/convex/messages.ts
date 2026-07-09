import { getWelcomeMessage, toTitleCase } from "@workspace/shared";
import { v } from "convex/values";

import { mutation, query } from "./_generated/server";

export function normalizeMessageBody(body: string) {
  return body.trim().replace(/\s+/g, " ");
}

export function buildSeedMessage(appName: string) {
  return normalizeMessageBody(getWelcomeMessage(toTitleCase(appName)));
}

export const list = query({
  args: {},
  handler: async (ctx) => await ctx.db.query("messages").order("desc").take(10),
});

export const seed = mutation({
  args: {
    appName: v.string(),
  },
  handler: async (ctx, args) => {
    const existing = await ctx.db.query("messages").take(1);

    if (existing.length) {
      return existing[0];
    }

    const body = buildSeedMessage(args.appName);
    const id = await ctx.db.insert("messages", {
      body,
      source: "seed",
    });

    return await ctx.db.get("messages", id);
  },
});
