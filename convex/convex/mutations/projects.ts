import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { buildProjectSlug, normalizeProjectKey } from "../lib/projects";
import { createDefaultIssueStates } from "../lib/states";
import { requireWorkspaceMembership } from "../lib/workspaceAccess";

export const generateUploadUrl = mutation({
  args: {},
  handler: async (ctx) => {
    await requireAuthUser(ctx);
    return await ctx.storage.generateUploadUrl();
  },
});

export const storeProjectImage = mutation({
  args: {
    projectId: v.id("projects"),
    storageId: v.string(),
    type: v.union(v.literal("cover"), v.literal("logo")),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await ctx.db.get(args.projectId);

    if (!project) {
      throw new ConvexError("Project not found");
    }

    await requireWorkspaceMembership(ctx, user._id, project.workspaceId);

    const url = (await ctx.storage.getUrl(args.storageId)) ?? undefined;

    const update: { coverImageUrl?: string; logoUrl?: string } = {};
    if (args.type === "cover") {
      update.coverImageUrl = url;
    } else {
      update.logoUrl = url;
    }

    await ctx.db.patch(args.projectId, update);

    return url;
  },
});

export const create = mutation({
  args: {
    color: v.optional(v.string()),
    coverImageUrl: v.optional(v.string()),
    description: v.optional(v.string()),
    key: v.string(),
    logoUrl: v.optional(v.string()),
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
      coverImageUrl: args.coverImageUrl,
      description: args.description?.trim() || undefined,
      key,
      logoUrl: args.logoUrl,
      name,
      slug: buildProjectSlug(name, key),
      updatedAt: Date.now(),
      workspaceId: args.workspaceId,
    });

    await createDefaultIssueStates(ctx, projectId, args.workspaceId);

    return await ctx.db.get(projectId);
  },
});
