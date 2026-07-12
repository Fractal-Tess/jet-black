import { defineTable } from "convex/server";
import { v } from "convex/values";

export const issuePriority = v.union(
  v.literal("none"),
  v.literal("low"),
  v.literal("medium"),
  v.literal("high"),
  v.literal("urgent")
);

export const issueStateType = v.union(
  v.literal("backlog"),
  v.literal("unstarted"),
  v.literal("started"),
  v.literal("completed"),
  v.literal("cancelled")
);

export const issueTables = {
  issueActivities: defineTable({
    actorUserId: v.string(),
    issueId: v.id("issues"),
    message: v.string(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_issueId", ["issueId"])
    .index("by_workspaceId", ["workspaceId"]),
  issueComments: defineTable({
    authorUserId: v.string(),
    body: v.string(),
    issueId: v.id("issues"),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_issueId", ["issueId"])
    .index("by_workspaceId", ["workspaceId"]),
  issues: defineTable({
    assigneeUserId: v.optional(v.string()),
    createdByUserId: v.string(),
    description: v.optional(v.string()),
    identifier: v.string(),
    priority: issuePriority,
    projectId: v.id("projects"),
    sequenceId: v.number(),
    stateId: v.id("issueStates"),
    title: v.string(),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_projectId_sequenceId", ["projectId", "sequenceId"])
    .index("by_workspaceId", ["workspaceId"])
    .searchIndex("search_title", {
      filterFields: ["projectId", "workspaceId"],
      searchField: "title",
    }),
  issueStates: defineTable({
    color: v.string(),
    isDefault: v.boolean(),
    name: v.string(),
    position: v.number(),
    projectId: v.id("projects"),
    type: issueStateType,
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_workspaceId", ["workspaceId"]),
};
