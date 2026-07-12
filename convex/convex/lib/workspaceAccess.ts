import { ConvexError } from "convex/values";

import type { Id } from "../_generated/dataModel";
import type { QueryCtx } from "../_generated/server";

type DbCtx = Pick<QueryCtx, "db">;

export async function requireWorkspaceMembership(
  ctx: DbCtx,
  userId: string,
  workspaceId: Id<"workspaces">
) {
  const membership = await ctx.db
    .query("workspaceMembers")
    .withIndex("by_workspaceId_userId", (q) =>
      q.eq("workspaceId", workspaceId).eq("userId", userId)
    )
    .unique();

  if (!membership) {
    throw new ConvexError("Workspace access denied");
  }

  return membership;
}

export async function requireProjectAccess(
  ctx: DbCtx,
  userId: string,
  projectId: Id<"projects">
) {
  const project = await ctx.db.get(projectId);

  if (!project) {
    throw new ConvexError("Project not found");
  }

  await requireWorkspaceMembership(ctx, userId, project.workspaceId);

  return project;
}

export async function requireIssueAccess(
  ctx: DbCtx,
  userId: string,
  issueId: Id<"issues">
) {
  const issue = await ctx.db.get(issueId);

  if (!issue) {
    throw new ConvexError("Issue not found");
  }

  await requireWorkspaceMembership(ctx, userId, issue.workspaceId);

  return issue;
}
