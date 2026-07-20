import { v } from "convex/values";

import { query } from "../_generated/server";
import { authComponent } from "../auth";
import { requireAuthUser } from "../lib/auth";
import { requireIssueAccess } from "../lib/workspaceAccess";

export const listForIssue = query({
  args: {
    issueId: v.id("issues"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireIssueAccess(ctx, user._id, args.issueId);

    const comments = await ctx.db
      .query("issueComments")
      .withIndex("by_issueId", (q) => q.eq("issueId", args.issueId))
      .order("asc")
      .collect();

    const authorIds = [
      ...new Set(comments.map((comment) => comment.authorUserId)),
    ];
    const authorEntries = await Promise.all(
      authorIds.map(
        async (authorId) =>
          [authorId, await authComponent.getAnyUserById(ctx, authorId)] as const
      )
    );
    const authorById = new Map(authorEntries);

    return comments.map((comment) => {
      const author = authorById.get(comment.authorUserId);

      return {
        ...comment,
        author: {
          image: author?.image ?? null,
          name: author?.name ?? "Former member",
        },
        isAuthor: comment.authorUserId === user._id,
      };
    });
  },
});
