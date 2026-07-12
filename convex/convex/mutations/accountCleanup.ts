import { v } from "convex/values";

import { internalMutation } from "../_generated/server";

export const deleteForUser = internalMutation({
  args: {
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    const memberships = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_userId", (q) => q.eq("userId", args.userId))
      .collect();
    const ownedWorkspaces = await ctx.db
      .query("workspaces")
      .withIndex("by_createdByUserId", (q) =>
        q.eq("createdByUserId", args.userId)
      )
      .collect();
    const workspaceIds = new Set([
      ...memberships.map((membership) => membership.workspaceId),
      ...ownedWorkspaces.map((workspace) => workspace._id),
    ]);

    for (const workspaceId of workspaceIds) {
      const issueComments = await ctx.db
        .query("issueComments")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      for (const comment of issueComments) {
        await ctx.db.delete(comment._id);
      }

      const issueActivities = await ctx.db
        .query("issueActivities")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      for (const activity of issueActivities) {
        await ctx.db.delete(activity._id);
      }

      const issues = await ctx.db
        .query("issues")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      for (const issue of issues) {
        await ctx.db.delete(issue._id);
      }

      const issueStates = await ctx.db
        .query("issueStates")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      for (const state of issueStates) {
        await ctx.db.delete(state._id);
      }

      const projects = await ctx.db
        .query("projects")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      for (const project of projects) {
        await ctx.db.delete(project._id);
      }

      const workspaceMembers = await ctx.db
        .query("workspaceMembers")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      for (const member of workspaceMembers) {
        await ctx.db.delete(member._id);
      }

      const workspace = await ctx.db.get(workspaceId);
      if (workspace?.createdByUserId === args.userId) {
        await ctx.db.delete(workspace._id);
      }
    }

    return { deleted: true };
  },
});
