import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireWorkspaceMembership } from "../lib/workspaceAccess";

export const generateUploadUrl = mutation({
  args: {
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceMembership(ctx, user._id, args.workspaceId);

    return await ctx.storage.generateUploadUrl();
  },
});

export const storeDescriptionAsset = mutation({
  args: {
    storageId: v.id("_storage"),
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceMembership(ctx, user._id, args.workspaceId);

    const url = await ctx.storage.getUrl(args.storageId);

    if (!url) {
      throw new ConvexError("Uploaded file not found");
    }

    return url;
  },
});
