import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireProjectAccess } from "../lib/workspaceAccess";

export const create = mutation({
  args: {
    content: v.optional(v.string()),
    icon: v.optional(v.string()),
    projectId: v.id("projects"),
    title: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const title = args.title.trim();

    if (!title) {
      throw new ConvexError("Page title is required");
    }

    const now = Date.now();

    return await ctx.db.insert("projectPages", {
      content: args.content?.trim() || "",
      createdAt: now,
      createdByUserId: user._id,
      icon: args.icon?.trim() || undefined,
      projectId: project._id,
      title,
      updatedAt: now,
      workspaceId: project.workspaceId,
    });
  },
});

export const update = mutation({
  args: {
    content: v.optional(v.string()),
    icon: v.optional(v.union(v.string(), v.null())),
    pageId: v.id("projectPages"),
    title: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const page = await ctx.db.get(args.pageId);

    if (!page) {
      throw new ConvexError("Page not found");
    }

    await requireProjectAccess(ctx, user._id, page.projectId);

    const patch: Partial<{
      content: string;
      icon: string | undefined;
      title: string;
      updatedAt: number;
    }> = { updatedAt: Date.now() };

    if (args.title !== undefined) {
      const title = args.title.trim();
      if (!title) {
        throw new ConvexError("Page title is required");
      }
      patch.title = title;
    }

    if (args.content !== undefined) {
      patch.content = args.content;
    }

    if (args.icon !== undefined) {
      patch.icon = args.icon?.trim() || undefined;
    }

    await ctx.db.patch(args.pageId, patch);

    return args.pageId;
  },
});
