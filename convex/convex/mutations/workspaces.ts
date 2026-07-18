import { v } from "convex/values";
import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  DEFAULT_PROJECT_KEY,
  DEFAULT_PROJECT_NAME,
} from "../lib/defaultWorkspace";
import { createSampleIssues } from "../lib/sampleIssues";
import { slugify } from "../lib/slugs";
import { createDefaultIssueStates } from "../lib/states";

export const createWorkspace = mutation({
  args: {
    name: v.string(),
    slug: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const now = Date.now();

    const existingSlug = await ctx.db
      .query("workspaces")
      .withIndex("by_slug", (q) => q.eq("slug", args.slug))
      .first();

    if (existingSlug) {
      throw new Error("A workspace with this URL already exists.");
    }

    const workspaceId = await ctx.db.insert("workspaces", {
      createdByUserId: user._id,
      name: args.name.trim(),
      slug: args.slug,
      updatedAt: now,
    });

    await ctx.db.insert("workspaceMembers", {
      role: "owner",
      userId: user._id,
      workspaceId,
    });

    return await ctx.db.get(workspaceId);
  },
});

export const ensurePersonalWorkspace = mutation({
  args: {},
  handler: async (ctx) => {
    const user = await requireAuthUser(ctx);
    const now = Date.now();
    const existingMembership = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_userId", (q) => q.eq("userId", user._id))
      .first();

    if (existingMembership) {
      const workspace = await ctx.db.get(existingMembership.workspaceId);
      const project = workspace
        ? await ctx.db
            .query("projects")
            .withIndex("by_workspaceId", (q) =>
              q.eq("workspaceId", workspace._id)
            )
            .first()
        : null;

      return { project, workspace };
    }

    const baseName = user.name?.trim() || "Jet Black";
    const slug = `${slugify(baseName) || "workspace"}-${user._id.slice(-6)}`;
    const workspaceId = await ctx.db.insert("workspaces", {
      createdByUserId: user._id,
      name: baseName,
      slug,
      updatedAt: now,
    });

    await ctx.db.insert("workspaceMembers", {
      role: "owner",
      userId: user._id,
      workspaceId,
    });

    const projectId = await ctx.db.insert("projects", {
      color: "#f59e0b",
      description: "A lean realtime project space for issues and code review.",
      key: DEFAULT_PROJECT_KEY,
      name: DEFAULT_PROJECT_NAME,
      slug: "jet-black",
      updatedAt: now,
      workspaceId,
    });

    await createDefaultIssueStates(ctx, projectId, workspaceId);
    await createSampleIssues(ctx, {
      actorUserId: user._id,
      projectId,
      projectKey: DEFAULT_PROJECT_KEY,
      workspaceId,
    });

    return {
      project: await ctx.db.get(projectId),
      workspace: await ctx.db.get(workspaceId),
    };
  },
});
