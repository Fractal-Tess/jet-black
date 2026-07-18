import { v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";

export const generateUploadUrl = mutation({
  args: {},
  handler: async (ctx) => {
    await requireAuthUser(ctx);
    return await ctx.storage.generateUploadUrl();
  },
});

export const getStorageUrl = mutation({
  args: {
    storageId: v.id("_storage"),
  },
  handler: async (ctx, args) => {
    await requireAuthUser(ctx);
    return await ctx.storage.getUrl(args.storageId);
  },
});
