import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireIssueAccess } from "../lib/workspaceAccess";

function normalizeUrl(url: string) {
  const normalizedUrl = url.trim();

  if (
    !(
      normalizedUrl.startsWith("https://") ||
      normalizedUrl.startsWith("http://")
    )
  ) {
    throw new ConvexError("Attachment URL must start with http:// or https://");
  }

  return normalizedUrl;
}

export const addLink = mutation({
  args: {
    issueId: v.id("issues"),
    name: v.string(),
    url: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const name = args.name.trim();

    if (!name) {
      throw new ConvexError("Attachment name is required");
    }

    const attachmentId = await ctx.db.insert("issueAttachments", {
      createdByUserId: user._id,
      issueId: issue._id,
      name,
      projectId: issue.projectId,
      url: normalizeUrl(args.url),
      workspaceId: issue.workspaceId,
    });

    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: issue._id,
      message: "added an attachment",
      workspaceId: issue.workspaceId,
    });

    return attachmentId;
  },
});
