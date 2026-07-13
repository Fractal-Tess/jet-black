import { defineTable } from "convex/server";
import { v } from "convex/values";

export const intakeStatus = v.union(
  v.literal("accepted"),
  v.literal("declined"),
  v.literal("pending"),
  v.literal("snoozed")
);

export const intakeTables = {
  intakeIssues: defineTable({
    acceptedIssueId: v.optional(v.id("issues")),
    createdAt: v.number(),
    createdByUserId: v.string(),
    description: v.optional(v.string()),
    projectId: v.id("projects"),
    source: v.string(),
    status: intakeStatus,
    title: v.string(),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_projectId_status", ["projectId", "status"])
    .index("by_workspaceId", ["workspaceId"]),
};
