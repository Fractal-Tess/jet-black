import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  requireIssueAccess,
  requireProjectAccess,
} from "../lib/workspaceAccess";

const DEFAULT_LABEL_COLOR = "#38bdf8";

export const create = mutation({
  args: {
    color: v.optional(v.string()),
    name: v.string(),
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const name = args.name.trim();

    if (!name) {
      throw new ConvexError("Label name is required");
    }

    const existingLabels = await ctx.db
      .query("issueLabels")
      .withIndex("by_projectId", (q) => q.eq("projectId", project._id))
      .collect();
    const existingLabel = existingLabels.find(
      (label) => label.name.toLowerCase() === name.toLowerCase()
    );

    if (existingLabel) {
      return existingLabel._id;
    }

    return await ctx.db.insert("issueLabels", {
      color: args.color?.trim() || DEFAULT_LABEL_COLOR,
      name,
      projectId: project._id,
      workspaceId: project.workspaceId,
    });
  },
});

export const update = mutation({
  args: {
    color: v.optional(v.string()),
    labelId: v.id("issueLabels"),
    name: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const label = await ctx.db.get(args.labelId);

    if (!label) {
      throw new ConvexError("Label not found");
    }

    await requireProjectAccess(ctx, user._id, label.projectId);

    const patch: { color?: string; name?: string } = {};

    if (args.name !== undefined) {
      const name = args.name.trim();

      if (!name) {
        throw new ConvexError("Label name is required");
      }

      const siblings = await ctx.db
        .query("issueLabels")
        .withIndex("by_projectId", (q) => q.eq("projectId", label.projectId))
        .collect();

      const duplicate = siblings.find(
        (sibling) =>
          sibling._id !== label._id &&
          sibling.name.toLowerCase() === name.toLowerCase()
      );

      if (duplicate) {
        throw new ConvexError("A label with this name already exists");
      }

      patch.name = name;
    }

    if (args.color !== undefined) {
      patch.color = args.color.trim() || DEFAULT_LABEL_COLOR;
    }

    await ctx.db.patch(label._id, patch);

    return null;
  },
});

const ASSIGNMENT_DELETE_BATCH = 200;

export const remove = mutation({
  args: {
    labelId: v.id("issueLabels"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const label = await ctx.db.get(args.labelId);

    if (!label) {
      throw new ConvexError("Label not found");
    }

    await requireProjectAccess(ctx, user._id, label.projectId);

    let assignments = await ctx.db
      .query("issueLabelAssignments")
      .withIndex("by_labelId", (q) => q.eq("labelId", label._id))
      .take(ASSIGNMENT_DELETE_BATCH);

    while (assignments.length > 0) {
      await Promise.all(
        assignments.map((assignment) => ctx.db.delete(assignment._id))
      );
      assignments = await ctx.db
        .query("issueLabelAssignments")
        .withIndex("by_labelId", (q) => q.eq("labelId", label._id))
        .take(ASSIGNMENT_DELETE_BATCH);
    }

    await ctx.db.delete(label._id);

    return null;
  },
});

export const toggleForIssue = mutation({
  args: {
    issueId: v.id("issues"),
    labelId: v.id("issueLabels"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const label = await ctx.db.get(args.labelId);

    if (!label || label.projectId !== issue.projectId) {
      throw new ConvexError("Invalid issue label");
    }

    const existingAssignment = await ctx.db
      .query("issueLabelAssignments")
      .withIndex("by_issueId_labelId", (q) =>
        q.eq("issueId", issue._id).eq("labelId", label._id)
      )
      .unique();

    if (existingAssignment) {
      await ctx.db.delete(existingAssignment._id);
    } else {
      await ctx.db.insert("issueLabelAssignments", {
        issueId: issue._id,
        labelId: label._id,
        projectId: issue.projectId,
        workspaceId: issue.workspaceId,
      });
    }

    await ctx.db.patch(issue._id, { updatedAt: Date.now() });
    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: issue._id,
      message: existingAssignment ? "removed a label" : "added a label",
      workspaceId: issue.workspaceId,
    });

    return { assigned: !existingAssignment };
  },
});
