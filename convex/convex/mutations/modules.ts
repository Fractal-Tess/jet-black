import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireProjectAccess } from "../lib/workspaceAccess";

export const create = mutation({
  args: {
    description: v.optional(v.string()),
    name: v.string(),
    projectId: v.id("projects"),
    status: v.optional(
      v.union(
        v.literal("backlog"),
        v.literal("planned"),
        v.literal("in_progress"),
        v.literal("completed")
      )
    ),
    targetDate: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const name = args.name.trim();

    if (!name) {
      throw new ConvexError("Module name is required");
    }

    const now = Date.now();

    return await ctx.db.insert("projectModules", {
      createdAt: now,
      createdByUserId: user._id,
      description: args.description?.trim() || undefined,
      name,
      projectId: project._id,
      status: args.status ?? "planned",
      targetDate: args.targetDate?.trim() || undefined,
      updatedAt: now,
      workspaceId: project.workspaceId,
    });
  },
});
