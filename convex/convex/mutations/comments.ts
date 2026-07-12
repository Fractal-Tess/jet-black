import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireIssueAccess } from "../lib/workspaceAccess";

export const create = mutation({
  args: {
    body: v.string(),
    issueId: v.id("issues"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const body = args.body.trim();

    if (!body) {
      throw new ConvexError("Comment body is required");
    }

    const commentId = await ctx.db.insert("issueComments", {
      authorUserId: user._id,
      body,
      issueId: args.issueId,
      updatedAt: Date.now(),
      workspaceId: issue.workspaceId,
    });

    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: args.issueId,
      message: "commented",
      workspaceId: issue.workspaceId,
    });

    return commentId;
  },
});
