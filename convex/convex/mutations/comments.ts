import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireIssueAccess } from "../lib/workspaceAccess";

export const create = mutation({
  args: {
    body: v.string(),
    issueId: v.id("issues"),
    parentCommentId: v.optional(v.id("issueComments")),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const body = args.body.trim();

    if (!body) {
      throw new ConvexError("Comment body is required");
    }

    let parentCommentId = args.parentCommentId;

    if (parentCommentId) {
      const parent = await ctx.db.get(parentCommentId);
      if (!parent || parent.issueId !== args.issueId) {
        throw new ConvexError("Parent comment not found");
      }
      // Threads stay one level deep: replying to a reply attaches to the
      // top-level comment.
      parentCommentId = parent.parentCommentId ?? parentCommentId;
    }

    const commentId = await ctx.db.insert("issueComments", {
      authorUserId: user._id,
      body,
      issueId: args.issueId,
      parentCommentId,
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

export const update = mutation({
  args: {
    body: v.string(),
    commentId: v.id("issueComments"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const comment = await ctx.db.get(args.commentId);

    if (!comment) {
      throw new ConvexError("Comment not found");
    }

    await requireIssueAccess(ctx, user._id, comment.issueId);

    if (comment.authorUserId !== user._id) {
      throw new ConvexError("Only the author can edit this comment");
    }

    const body = args.body.trim();

    if (!body) {
      throw new ConvexError("Comment body is required");
    }

    await ctx.db.patch(args.commentId, {
      body,
      editedAt: Date.now(),
      updatedAt: Date.now(),
    });

    return args.commentId;
  },
});

export const remove = mutation({
  args: {
    commentId: v.id("issueComments"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const comment = await ctx.db.get(args.commentId);

    if (!comment) {
      throw new ConvexError("Comment not found");
    }

    await requireIssueAccess(ctx, user._id, comment.issueId);

    if (comment.authorUserId !== user._id) {
      throw new ConvexError("Only the author can delete this comment");
    }

    // Replies would be orphaned without their parent, so they go too.
    const replies = await ctx.db
      .query("issueComments")
      .withIndex("by_issueId", (q) => q.eq("issueId", comment.issueId))
      .filter((q) => q.eq(q.field("parentCommentId"), args.commentId))
      .collect();

    await Promise.all(replies.map((reply) => ctx.db.delete(reply._id)));
    await ctx.db.delete(args.commentId);
  },
});
