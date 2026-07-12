import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { buildProjectSlug, normalizeProjectKey } from "../lib/projects";
import { createDefaultIssueStates } from "../lib/states";
import { requireWorkspaceMembership } from "../lib/workspaceAccess";

export const create = mutation({
  args: {
    color: v.optional(v.string()),
    description: v.optional(v.string()),
    key: v.string(),
    name: v.string(),
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceMembership(ctx, user._id, args.workspaceId);

    const name = args.name.trim();
    const key = normalizeProjectKey(args.key);

    if (!name) {
      throw new ConvexError("Project name is required");
    }

    if (!key) {
      throw new ConvexError("Project key is required");
    }

    const existingProject = await ctx.db
      .query("projects")
      .withIndex("by_workspaceId_key", (q) =>
        q.eq("workspaceId", args.workspaceId).eq("key", key)
      )
      .unique();

    if (existingProject) {
      throw new ConvexError("Project key is already in use");
    }

    const projectId = await ctx.db.insert("projects", {
      color: args.color ?? "#38bdf8",
      description: args.description?.trim() || undefined,
      key,
      name,
      slug: buildProjectSlug(name, key),
      updatedAt: Date.now(),
      workspaceId: args.workspaceId,
    });

    await createDefaultIssueStates(ctx, projectId, args.workspaceId);

    return await ctx.db.get(projectId);
  },
});
