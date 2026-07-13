import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireProjectAccess } from "../lib/workspaceAccess";

export const create = mutation({
  args: {
    description: v.optional(v.string()),
    endDate: v.optional(v.string()),
    name: v.string(),
    projectId: v.id("projects"),
    startDate: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const name = args.name.trim();

    if (!name) {
      throw new ConvexError("Sprint name is required");
    }

    const now = Date.now();

    return await ctx.db.insert("sprints", {
      createdAt: now,
      createdByUserId: user._id,
      description: args.description?.trim() || undefined,
      endDate: args.endDate?.trim() || undefined,
      name,
      projectId: project._id,
      startDate: args.startDate?.trim() || undefined,
      updatedAt: now,
      workspaceId: project.workspaceId,
    });
  },
});
