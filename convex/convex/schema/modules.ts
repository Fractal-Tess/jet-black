import { defineTable } from "convex/server";
import { v } from "convex/values";

export const moduleTables = {
  projectModules: defineTable({
    createdAt: v.number(),
    createdByUserId: v.string(),
    description: v.optional(v.string()),
    leadUserId: v.optional(v.string()),
    name: v.string(),
    projectId: v.id("projects"),
    status: v.union(
      v.literal("backlog"),
      v.literal("planned"),
      v.literal("in_progress"),
      v.literal("completed")
    ),
    targetDate: v.optional(v.string()),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_workspaceId", ["workspaceId"]),
};
