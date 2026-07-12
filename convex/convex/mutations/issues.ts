import { ConvexError, v } from "convex/values";

import type { Id } from "../_generated/dataModel";
import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { getDefaultStateId, getNextSequenceId } from "../lib/issues";
import {
  requireIssueAccess,
  requireProjectAccess,
} from "../lib/workspaceAccess";

const priorityValidator = v.union(
  v.literal("none"),
  v.literal("low"),
  v.literal("medium"),
  v.literal("high"),
  v.literal("urgent")
);

export const create = mutation({
  args: {
    description: v.optional(v.string()),
    priority: priorityValidator,
    projectId: v.id("projects"),
    title: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const title = args.title.trim();

    if (!title) {
      throw new ConvexError("Issue title is required");
    }

    const now = Date.now();
    const sequenceId = await getNextSequenceId(ctx, project._id);
    const stateId = await getDefaultStateId(ctx, project._id);
    const issueId = await ctx.db.insert("issues", {
      createdByUserId: user._id,
      description: args.description?.trim() || undefined,
      identifier: `${project.key}-${sequenceId}`,
      priority: args.priority,
      projectId: project._id,
      sequenceId,
      stateId,
      title,
      updatedAt: now,
      workspaceId: project.workspaceId,
    });

    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId,
      message: "created the issue",
      workspaceId: project.workspaceId,
    });

    return issueId;
  },
});

export const update = mutation({
  args: {
    description: v.optional(v.string()),
    issueId: v.id("issues"),
    priority: v.optional(priorityValidator),
    stateId: v.optional(v.id("issueStates")),
    title: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const patch: Partial<{
      description: string | undefined;
      priority: "none" | "low" | "medium" | "high" | "urgent";
      stateId: Id<"issueStates">;
      title: string;
      updatedAt: number;
    }> = { updatedAt: Date.now() };

    if (args.title !== undefined) {
      const title = args.title.trim();
      if (!title) {
        throw new ConvexError("Issue title is required");
      }
      patch.title = title;
    }

    if (args.description !== undefined) {
      patch.description = args.description.trim() || undefined;
    }

    if (args.priority !== undefined) {
      patch.priority = args.priority;
    }

    if (args.stateId !== undefined) {
      const state = await ctx.db.get(args.stateId);
      if (!state || state.projectId !== issue.projectId) {
        throw new ConvexError("Invalid issue state");
      }
      patch.stateId = args.stateId;
    }

    await ctx.db.patch(args.issueId, patch);
    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: args.issueId,
      message: "updated the issue",
      workspaceId: issue.workspaceId,
    });

    return args.issueId;
  },
});
