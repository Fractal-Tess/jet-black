import { defineTable } from "convex/server";
import { v } from "convex/values";

export const workspaceTables = {
  projects: defineTable({
    color: v.string(),
    description: v.optional(v.string()),
    key: v.string(),
    name: v.string(),
    slug: v.string(),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_key", ["workspaceId", "key"]),
  workspaceMembers: defineTable({
    role: v.union(v.literal("owner"), v.literal("member")),
    userId: v.string(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_userId", ["userId"])
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_userId", ["workspaceId", "userId"]),
  workspaces: defineTable({
    createdByUserId: v.string(),
    name: v.string(),
    slug: v.string(),
    updatedAt: v.number(),
  })
    .index("by_createdByUserId", ["createdByUserId"])
    .index("by_slug", ["slug"]),
};
