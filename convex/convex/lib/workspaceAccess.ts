import { ConvexError } from "convex/values";

import type { Id } from "../_generated/dataModel";
import type { QueryCtx } from "../_generated/server";

type DbCtx = Pick<QueryCtx, "db">;

export type WorkspaceRole = "owner" | "admin" | "member" | "guest";

export const ROLE_RANK: Record<WorkspaceRole, number> = {
  admin: 2,
  guest: 0,
  member: 1,
  owner: 3,
};

export function roleAtLeast(role: WorkspaceRole, minimum: WorkspaceRole) {
  return ROLE_RANK[role] >= ROLE_RANK[minimum];
}

export async function requireWorkspaceRole(
  ctx: DbCtx,
  userId: string,
  workspaceId: Id<"workspaces">,
  minimum: WorkspaceRole
) {
  const membership = await requireWorkspaceMembership(ctx, userId, workspaceId);

  if (!roleAtLeast(membership.role, minimum)) {
    throw new ConvexError("You do not have permission to perform this action");
  }

  return membership;
}

export async function requireProjectAdmin(
  ctx: DbCtx,
  userId: string,
  projectId: Id<"projects">
) {
  const project = await ctx.db.get(projectId);

  if (!project) {
    throw new ConvexError("Project not found");
  }

  await requireWorkspaceRole(ctx, userId, project.workspaceId, "admin");

  return project;
}

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
