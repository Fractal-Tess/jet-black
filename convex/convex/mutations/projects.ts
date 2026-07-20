import { ConvexError, v } from "convex/values";

import { internal } from "../_generated/api";
import { internalMutation, mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { buildProjectSlug, normalizeProjectKey } from "../lib/projects";
import { createDefaultIssueStates } from "../lib/states";
import {
  requireProjectAdmin,
  requireWorkspaceMembership,
} from "../lib/workspaceAccess";
import {
  estimateSystemValidator,
  projectAutomationsValidator,
  projectFeaturesValidator,
} from "../schema/workspaces";

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

export const update = mutation({
  args: {
    color: v.optional(v.string()),
    description: v.optional(v.union(v.string(), v.null())),
    key: v.optional(v.string()),
    name: v.optional(v.string()),
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    const name = args.name === undefined ? project.name : args.name.trim();
    const key =
      args.key === undefined ? project.key : normalizeProjectKey(args.key);

    if (!name) {
      throw new ConvexError("Project name is required");
    }

    if (!key) {
      throw new ConvexError("Project key is required");
    }

    if (key !== project.key) {
      const existingProject = await ctx.db
        .query("projects")
        .withIndex("by_workspaceId_key", (q) =>
          q.eq("workspaceId", project.workspaceId).eq("key", key)
        )
        .unique();

      if (existingProject && existingProject._id !== project._id) {
        throw new ConvexError("Project key is already in use");
      }
    }

    const patch: {
      color?: string;
      description?: string | undefined;
      key: string;
      name: string;
      slug: string;
      updatedAt: number;
    } = {
      key,
      name,
      slug: buildProjectSlug(name, key),
      updatedAt: Date.now(),
    };

    if (args.color !== undefined) {
      patch.color = args.color;
    }

    if (args.description !== undefined) {
      patch.description = args.description?.trim() || undefined;
    }

    await ctx.db.patch(project._id, patch);

    return await ctx.db.get(project._id);
  },
});

export const archive = mutation({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    await ctx.db.patch(project._id, {
      archivedAt: Date.now(),
      updatedAt: Date.now(),
    });

    return null;
  },
});

export const restore = mutation({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    await ctx.db.patch(project._id, {
      archivedAt: undefined,
      updatedAt: Date.now(),
    });

    return null;
  },
});

export const updateFeatures = mutation({
  args: {
    features: projectFeaturesValidator,
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    await ctx.db.patch(project._id, {
      features: args.features,
      updatedAt: Date.now(),
    });

    return null;
  },
});

export const updateEstimateSystem = mutation({
  args: {
    estimateSystem: v.union(estimateSystemValidator, v.null()),
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    if (args.estimateSystem && args.estimateSystem.values.length === 0) {
      throw new ConvexError("Estimate systems need at least one value");
    }

    await ctx.db.patch(project._id, {
      estimateSystem: args.estimateSystem ?? undefined,
      updatedAt: Date.now(),
    });

    return null;
  },
});

export const updateAutomations = mutation({
  args: {
    automations: projectAutomationsValidator,
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    await ctx.db.patch(project._id, {
      automations: args.automations,
      updatedAt: Date.now(),
    });

    return null;
  },
});

export const remove = mutation({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    await ctx.db.delete(project._id);

    await ctx.scheduler.runAfter(
      0,
      internal.mutations.projects.cascadeDeleteProjectData,
      { projectId: args.projectId }
    );

    return null;
  },
});

const PROJECT_CASCADE_BATCH = 500;

const PROJECT_CASCADE_TABLES = [
  "intakeIssues",
  "issueAttachments",
  "issueLabelAssignments",
  "issueLabels",
  "issueStates",
  "projectMembers",
  "projectModuleIssueAssignments",
  "projectModuleLinks",
  "projectModuleMembers",
  "projectModules",
  "projectPages",
  "sprints",
] as const;

export const cascadeDeleteProjectData = internalMutation({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    let deleted = 0;

    // Issues first: their comments/activities are only reachable by issueId.
    const issues = await ctx.db
      .query("issues")
      .withIndex("by_projectId", (q) => q.eq("projectId", args.projectId))
      .take(50);

    for (const issue of issues) {
      const comments = await ctx.db
        .query("issueComments")
        .withIndex("by_issueId", (q) => q.eq("issueId", issue._id))
        .take(PROJECT_CASCADE_BATCH);
      const activities = await ctx.db
        .query("issueActivities")
        .withIndex("by_issueId", (q) => q.eq("issueId", issue._id))
        .take(PROJECT_CASCADE_BATCH);

      await Promise.all([
        ...comments.map((row) => ctx.db.delete(row._id)),
        ...activities.map((row) => ctx.db.delete(row._id)),
      ]);

      deleted += comments.length + activities.length;

      if (
        comments.length < PROJECT_CASCADE_BATCH &&
        activities.length < PROJECT_CASCADE_BATCH
      ) {
        await ctx.db.delete(issue._id);
        deleted += 1;
      }

      if (deleted >= PROJECT_CASCADE_BATCH) {
        break;
      }
    }

    for (const table of PROJECT_CASCADE_TABLES) {
      if (deleted >= PROJECT_CASCADE_BATCH) {
        break;
      }

      const rows = await ctx.db
        .query(table)
        .withIndex("by_projectId", (q) => q.eq("projectId", args.projectId))
        .take(PROJECT_CASCADE_BATCH - deleted);

      await Promise.all(rows.map((row) => ctx.db.delete(row._id)));
      deleted += rows.length;
    }

    if (deleted >= PROJECT_CASCADE_BATCH || issues.length > 0) {
      await ctx.scheduler.runAfter(
        0,
        internal.mutations.projects.cascadeDeleteProjectData,
        { projectId: args.projectId }
      );
    }
  },
});
