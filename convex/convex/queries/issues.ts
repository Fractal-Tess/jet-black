import { v } from "convex/values";

import type { Id } from "../_generated/dataModel";
import type { QueryCtx } from "../_generated/server";
import { query } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  requireIssueAccess,
  requireProjectAccess,
} from "../lib/workspaceAccess";

async function labelsForIssue(ctx: QueryCtx, issueId: Id<"issues">) {
  const assignments = await ctx.db
    .query("issueLabelAssignments")
    .withIndex("by_issueId", (q) => q.eq("issueId", issueId))
    .collect();
  const labels = await Promise.all(
    assignments.map(async (assignment) => await ctx.db.get(assignment.labelId))
  );

  return labels.filter((label) => label !== null);
}

export const listForProject = query({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireProjectAccess(ctx, user._id, args.projectId);

    const issues = await ctx.db
      .query("issues")
      .withIndex("by_projectId_sequenceId", (q) =>
        q.eq("projectId", args.projectId)
      )
      .order("desc")
      .collect();

    return await Promise.all(
      issues.map(async (issue) => ({
        ...issue,
        labels: await labelsForIssue(ctx, issue._id),
        state: await ctx.db.get(issue.stateId),
      }))
    );
  },
});

export const get = query({
  args: {
    issueId: v.id("issues"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);

    return {
      ...issue,
      labels: await labelsForIssue(ctx, issue._id),
      project: await ctx.db.get(issue.projectId),
      state: await ctx.db.get(issue.stateId),
    };
  },
});
