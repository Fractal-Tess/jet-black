import { v } from "convex/values";

import { query } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireIssueAccess } from "../lib/workspaceAccess";

export const listForIssue = query({
  args: {
    issueId: v.id("issues"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireIssueAccess(ctx, user._id, args.issueId);

    return await ctx.db
      .query("issueComments")
      .withIndex("by_issueId", (q) => q.eq("issueId", args.issueId))
      .order("asc")
      .collect();
  },
});
