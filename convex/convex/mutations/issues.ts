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
    parentIssueId: v.optional(v.id("issues")),
    priority: priorityValidator,
    projectId: v.id("projects"),
    stateId: v.optional(v.id("issueStates")),
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
    const stateId = args.stateId ?? (await getDefaultStateId(ctx, project._id));
    const state = await ctx.db.get(stateId);
    const parentIssue = args.parentIssueId
      ? await ctx.db.get(args.parentIssueId)
      : null;

    if (!state || state.projectId !== project._id) {
      throw new ConvexError("Invalid issue state");
    }

    if (args.parentIssueId && parentIssue?.projectId !== project._id) {
      throw new ConvexError("Invalid parent issue");
    }

    const issueId = await ctx.db.insert("issues", {
      createdByUserId: user._id,
      createdAt: now,
      description: args.description?.trim() || undefined,
      identifier: `${project.key}-${sequenceId}`,
      parentIssueId: args.parentIssueId,
      position: now,
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
      message: args.parentIssueId ? "created a sub-issue" : "created the issue",
      workspaceId: project.workspaceId,
    });

    return issueId;
  },
});

export const update = mutation({
  args: {
    description: v.optional(v.string()),
    estimate: v.optional(v.union(v.number(), v.null())),
    issueId: v.id("issues"),
    priority: v.optional(priorityValidator),
    stateId: v.optional(v.id("issueStates")),
    startDate: v.optional(v.union(v.string(), v.null())),
    targetDate: v.optional(v.union(v.string(), v.null())),
    title: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const patch: Partial<{
      description: string | undefined;
      completedAt: number | undefined;
      estimate: number | undefined;
      priority: "none" | "low" | "medium" | "high" | "urgent";
      stateId: Id<"issueStates">;
      startDate: string | undefined;
      targetDate: string | undefined;
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

    if (args.estimate !== undefined) {
      patch.estimate = args.estimate ?? undefined;
    }

    if (args.startDate !== undefined) {
      patch.startDate = args.startDate?.trim() || undefined;
    }

    if (args.targetDate !== undefined) {
      patch.targetDate = args.targetDate?.trim() || undefined;
    }

    if (args.stateId !== undefined) {
      const state = await ctx.db.get(args.stateId);
      if (!state || state.projectId !== issue.projectId) {
        throw new ConvexError("Invalid issue state");
      }
      patch.stateId = args.stateId;
      patch.completedAt =
        state.type === "completed"
          ? (issue.completedAt ?? Date.now())
          : undefined;
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

export const move = mutation({
  args: {
    issueId: v.id("issues"),
    position: v.number(),
    stateId: v.id("issueStates"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const state = await ctx.db.get(args.stateId);

    if (!state || state.projectId !== issue.projectId) {
      throw new ConvexError("Invalid issue state");
    }

    await ctx.db.patch(args.issueId, {
      completedAt:
        state.type === "completed"
          ? (issue.completedAt ?? Date.now())
          : undefined,
      position: args.position,
      stateId: args.stateId,
      updatedAt: Date.now(),
    });
    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: args.issueId,
      message:
        args.stateId === issue.stateId
          ? "reordered the issue"
          : "moved the issue",
      workspaceId: issue.workspaceId,
    });

    return args.issueId;
  },
});

export const archive = mutation({
  args: {
    issueId: v.id("issues"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);

    if (issue.archivedAt) {
      return args.issueId;
    }

    await ctx.db.patch(args.issueId, {
      archivedAt: Date.now(),
      updatedAt: Date.now(),
    });
    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: args.issueId,
      message: "archived the issue",
      workspaceId: issue.workspaceId,
    });

    return args.issueId;
  },
});
