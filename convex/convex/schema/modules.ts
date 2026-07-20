import { defineTable } from "convex/server";
import { v } from "convex/values";

export const moduleStatusValidator = v.union(
  v.literal("backlog"),
  v.literal("planned"),
  v.literal("in_progress"),
  v.literal("paused"),
  v.literal("completed"),
  v.literal("cancelled")
);

export const moduleTables = {
  projectModuleIssueAssignments: defineTable({
    assignedAt: v.number(),
    assignedByUserId: v.string(),
    issueId: v.id("issues"),
    moduleId: v.id("projectModules"),
    projectId: v.id("projects"),
    workspaceId: v.id("workspaces"),
  })
    .index("by_issueId", ["issueId"])
    .index("by_issueId_and_moduleId", ["issueId", "moduleId"])
    .index("by_moduleId", ["moduleId"])
    .index("by_moduleId_and_issueId", ["moduleId", "issueId"])
    .index("by_projectId", ["projectId"])
    .index("by_projectId_and_issueId", ["projectId", "issueId"])
    .index("by_projectId_and_moduleId", ["projectId", "moduleId"])
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_and_issueId", ["workspaceId", "issueId"])
    .index("by_workspaceId_and_moduleId", ["workspaceId", "moduleId"]),
  projectModuleLinks: defineTable({
    createdAt: v.number(),
    createdByUserId: v.string(),
    moduleId: v.id("projectModules"),
    projectId: v.id("projects"),
    title: v.string(),
    updatedAt: v.number(),
    url: v.string(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_moduleId", ["moduleId"])
    .index("by_moduleId_and_url", ["moduleId", "url"])
    .index("by_projectId", ["projectId"])
    .index("by_projectId_and_moduleId", ["projectId", "moduleId"])
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_and_moduleId", ["workspaceId", "moduleId"]),
  projectModuleMembers: defineTable({
    moduleId: v.id("projectModules"),
    projectId: v.id("projects"),
    userId: v.string(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_moduleId", ["moduleId"])
    .index("by_moduleId_and_userId", ["moduleId", "userId"])
    .index("by_projectId", ["projectId"])
    .index("by_projectId_and_moduleId", ["projectId", "moduleId"])
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_and_moduleId", ["workspaceId", "moduleId"])
    .index("by_workspaceId_and_userId", ["workspaceId", "userId"]),
  projectModules: defineTable({
    archivedAt: v.optional(v.number()),
    createdAt: v.number(),
    createdByUserId: v.string(),
    description: v.optional(v.string()),
    leadUserId: v.optional(v.string()),
    name: v.string(),
    projectId: v.id("projects"),
    startDate: v.optional(v.string()),
    status: moduleStatusValidator,
    targetDate: v.optional(v.string()),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_projectId_and_archivedAt", ["projectId", "archivedAt"])
    .index("by_workspaceId", ["workspaceId"]),
};
