import { defineTable } from "convex/server";
import { v } from "convex/values";

export const sprintTables = {
  sprints: defineTable({
    createdAt: v.number(),
    createdByUserId: v.string(),
    description: v.optional(v.string()),
    endDate: v.optional(v.string()),
    name: v.string(),
    projectId: v.id("projects"),
    startDate: v.optional(v.string()),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_workspaceId", ["workspaceId"]),
};
