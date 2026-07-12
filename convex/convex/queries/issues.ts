import { v } from "convex/values";

import { query } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  requireIssueAccess,
  requireProjectAccess,
} from "../lib/workspaceAccess";

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
      project: await ctx.db.get(issue.projectId),
      state: await ctx.db.get(issue.stateId),
    };
  },
});
