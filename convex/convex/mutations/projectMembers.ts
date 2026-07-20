import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  requireProjectAdmin,
  requireWorkspaceMembership,
} from "../lib/workspaceAccess";
import { projectMemberRole } from "../schema/workspaces";

export const add = mutation({
  args: {
    projectId: v.id("projects"),
    role: projectMemberRole,
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    // The target must already belong to the workspace.
    await requireWorkspaceMembership(ctx, args.userId, project.workspaceId);

    const existing = await ctx.db
      .query("projectMembers")
      .withIndex("by_projectId_userId", (q) =>
        q.eq("projectId", project._id).eq("userId", args.userId)
      )
      .unique();

    if (existing) {
      throw new ConvexError("This person is already a project member");
    }

    return await ctx.db.insert("projectMembers", {
      projectId: project._id,
      role: args.role,
      userId: args.userId,
      workspaceId: project.workspaceId,
    });
  },
});

export const updateRole = mutation({
  args: {
    projectId: v.id("projects"),
    role: projectMemberRole,
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    const membership = await ctx.db
      .query("projectMembers")
      .withIndex("by_projectId_userId", (q) =>
        q.eq("projectId", project._id).eq("userId", args.userId)
      )
      .unique();

    if (!membership) {
      throw new ConvexError("Project member not found");
    }

    await ctx.db.patch(membership._id, { role: args.role });

    return null;
  },
});

export const remove = mutation({
  args: {
    projectId: v.id("projects"),
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAdmin(ctx, user._id, args.projectId);

    const membership = await ctx.db
      .query("projectMembers")
      .withIndex("by_projectId_userId", (q) =>
        q.eq("projectId", project._id).eq("userId", args.userId)
      )
      .unique();

    if (!membership) {
      throw new ConvexError("Project member not found");
    }

    await ctx.db.delete(membership._id);

    return null;
  },
});
