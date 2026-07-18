import { v } from "convex/values";

import type { Id } from "../_generated/dataModel";
import type { MutationCtx } from "../_generated/server";
import { internalMutation } from "../_generated/server";

const CASCADE_TABLES = [
  "issueComments",
  "issueAttachments",
  "intakeIssues",
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
        await ctx.db.delete(membership._id);
      }
    }

    return { deleted: true };
  },
});
