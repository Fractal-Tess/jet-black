import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { getDefaultStateId, getNextSequenceId } from "../lib/issues";
import { requireProjectAccess } from "../lib/workspaceAccess";
import { intakeStatus } from "../schema/intake";
import { issuePriority } from "../schema/issues";

function normalizeRequired(value: string, message: string) {
  const normalized = value.trim();

  if (!normalized) {
    throw new ConvexError(message);
  }

  return normalized;
}

export const create = mutation({
  args: {
    description: v.optional(v.string()),
    projectId: v.id("projects"),
    source: v.optional(v.string()),
    title: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const now = Date.now();

    return await ctx.db.insert("intakeIssues", {
      createdAt: now,
      createdByUserId: user._id,
      description: args.description?.trim() || undefined,
      projectId: project._id,
      source: args.source?.trim() || "manual",
      status: "pending",
      title: normalizeRequired(args.title, "Intake title is required"),
      updatedAt: now,
      workspaceId: project.workspaceId,
    });
  },
});

export const updateStatus = mutation({
  args: {
    intakeIssueId: v.id("intakeIssues"),
    status: intakeStatus,
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const intakeIssue = await ctx.db.get(args.intakeIssueId);

    if (!intakeIssue) {
      throw new ConvexError("Intake issue not found");
    }

    await requireProjectAccess(ctx, user._id, intakeIssue.projectId);

    if (intakeIssue.status === "accepted") {
      throw new ConvexError("Accepted intake items cannot be updated");
    }

    await ctx.db.patch(intakeIssue._id, {
      status: args.status,
      updatedAt: Date.now(),
    });

    return intakeIssue._id;
  },
});

export const accept = mutation({
  args: {
    intakeIssueId: v.id("intakeIssues"),
    priority: v.optional(issuePriority),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const intakeIssue = await ctx.db.get(args.intakeIssueId);

    if (!intakeIssue) {
      throw new ConvexError("Intake issue not found");
    }

    const project = await requireProjectAccess(
      ctx,
      user._id,
      intakeIssue.projectId
    );

    if (intakeIssue.status === "accepted") {
      return intakeIssue.acceptedIssueId;
    }

    const now = Date.now();
    const sequenceId = await getNextSequenceId(ctx, project._id);
    const stateId = await getDefaultStateId(ctx, project._id);
    const issueId = await ctx.db.insert("issues", {
      createdAt: now,
      createdByUserId: user._id,
      description: intakeIssue.description,
      identifier: `${project.key}-${sequenceId}`,
      position: now,
      priority: args.priority ?? "medium",
      projectId: project._id,
      sequenceId,
      stateId,
      title: intakeIssue.title,
      updatedAt: now,
      workspaceId: project.workspaceId,
    });

    await ctx.db.patch(intakeIssue._id, {
      acceptedIssueId: issueId,
      status: "accepted",
      updatedAt: now,
    });
    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId,
      message: "accepted from intake",
      workspaceId: project.workspaceId,
    });

    return issueId;
  },
});
