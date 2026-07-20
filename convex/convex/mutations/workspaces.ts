import { ConvexError, v } from "convex/values";
import { internal } from "../_generated/api";
import { internalMutation, mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  DEFAULT_PROJECT_KEY,
  DEFAULT_PROJECT_NAME,
} from "../lib/defaultWorkspace";
import { createSampleIssues } from "../lib/sampleIssues";
import { slugify } from "../lib/slugs";
import { createDefaultIssueStates } from "../lib/states";
import { requireWorkspaceRole } from "../lib/workspaceAccess";

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

export const rename = mutation({
  args: {
    name: v.string(),
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceRole(ctx, user._id, args.workspaceId, "admin");

    const name = args.name.trim();

    if (!name) {
      throw new ConvexError("Workspace name is required");
    }

    await ctx.db.patch(args.workspaceId, {
      name,
      updatedAt: Date.now(),
    });

    return await ctx.db.get(args.workspaceId);
  },
});

export const remove = mutation({
  args: {
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceRole(ctx, user._id, args.workspaceId, "owner");

    const memberships = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_workspaceId", (q) => q.eq("workspaceId", args.workspaceId))
      .collect();
    const invites = await ctx.db
      .query("workspaceInvites")
      .withIndex("by_workspaceId", (q) => q.eq("workspaceId", args.workspaceId))
      .collect();

    await Promise.all([
      ...memberships.map((membership) => ctx.db.delete(membership._id)),
      ...invites.map((invite) => ctx.db.delete(invite._id)),
    ]);

    await ctx.db.delete(args.workspaceId);

    await ctx.scheduler.runAfter(
      0,
      internal.mutations.workspaces.cascadeDeleteWorkspaceData,
      { workspaceId: args.workspaceId }
    );

    return null;
  },
});

const CASCADE_BATCH = 500;

const CASCADE_TABLES = [
  "intakeIssues",
  "issueActivities",
  "issueAttachments",
  "issueComments",
  "issueLabelAssignments",
  "issueLabels",
  "issueStates",
  "issues",
  "projectMembers",
  "projectModuleIssueAssignments",
  "projectModuleLinks",
  "projectModuleMembers",
  "projectModules",
  "projectPages",
  "projects",
  "sprints",
] as const;

export const cascadeDeleteWorkspaceData = internalMutation({
  args: {
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    let deleted = 0;

    for (const table of CASCADE_TABLES) {
      if (deleted >= CASCADE_BATCH) {
        break;
      }

      const rows = await ctx.db
        .query(table)
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", args.workspaceId)
        )
        .take(CASCADE_BATCH - deleted);

      await Promise.all(rows.map((row) => ctx.db.delete(row._id)));
      deleted += rows.length;
    }

    if (deleted >= CASCADE_BATCH) {
      await ctx.scheduler.runAfter(
        0,
        internal.mutations.workspaces.cascadeDeleteWorkspaceData,
        { workspaceId: args.workspaceId }
      );
    }
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
