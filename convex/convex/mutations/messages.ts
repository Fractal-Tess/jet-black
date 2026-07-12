import { v } from "convex/values";

import { mutation } from "../_generated/server";
import { buildSeedMessage } from "../lib/messages";

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
