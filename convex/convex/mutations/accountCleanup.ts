import { v } from "convex/values";

import type { Id } from "../_generated/dataModel";
import type { MutationCtx } from "../_generated/server";
import { internalMutation } from "../_generated/server";

const CASCADE_TABLES = [
  "issueComments",
  "issueAttachments",
  "intakeIssues",
  "projectModuleIssueAssignments",
  "projectModuleLinks",
  "projectModuleMembers",
  "projectModules",
  "projectPages",
  "sprints",
  "issueActivities",
  "issueLabelAssignments",
  "issueLabels",
  "issues",
  "issueStates",
  "projects",
  "workspaceMembers",
] as const;

async function removeUserFromWorkspaceModules(
  ctx: MutationCtx,
  workspaceId: Id<"workspaces">,
  userId: string
) {
  const moduleMembers = await ctx.db
    .query("projectModuleMembers")
    .withIndex("by_workspaceId_and_userId", (q) =>
      q.eq("workspaceId", workspaceId).eq("userId", userId)
    )
    .collect();
  for (const moduleMember of moduleMembers) {
    await ctx.db.delete(moduleMember._id);
  }

  const projectModules = await ctx.db
    .query("projectModules")
    .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
    .collect();
  for (const projectModule of projectModules) {
    if (projectModule.leadUserId === userId) {
      await ctx.db.patch(projectModule._id, {
        leadUserId: undefined,
        updatedAt: Date.now(),
      });
    }
  }
}

async function deleteWorkspaceData(
  ctx: MutationCtx,
  workspaceId: Id<"workspaces">
) {
  for (const table of CASCADE_TABLES) {
    const records = await ctx.db
      .query(table)
      .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
      .collect();

    for (const record of records) {
      await ctx.db.delete(record._id);
    }
  }

  await ctx.db.delete(workspaceId);
}

export const deleteForUser = internalMutation({
  args: {
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    const memberships = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_userId", (q) => q.eq("userId", args.userId))
      .collect();

    for (const membership of memberships) {
      const allMembers = await ctx.db
        .query("workspaceMembers")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", membership.workspaceId)
        )
        .collect();

      const isOwner = membership.role === "owner";
      const otherMembers = allMembers.filter((m) => m.userId !== args.userId);

      if (isOwner && otherMembers.length > 0) {
        throw new Error(
          "Cannot delete account while you own a workspace with other members. Transfer ownership or remove all members first."
        );
      }

      if (isOwner) {
        await deleteWorkspaceData(ctx, membership.workspaceId);
      } else {
        await removeUserFromWorkspaceModules(
          ctx,
          membership.workspaceId,
          args.userId
        );
        await ctx.db.delete(membership._id);
      }
    }

    return { deleted: true };
  },
});
