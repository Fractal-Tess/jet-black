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
  issueAttachments: defineTable({
    createdByUserId: v.string(),
    issueId: v.id("issues"),
    name: v.string(),
    projectId: v.id("projects"),
    url: v.string(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_issueId", ["issueId"])
    .index("by_projectId", ["projectId"])
    .index("by_workspaceId", ["workspaceId"]),
  issueLabelAssignments: defineTable({
    issueId: v.id("issues"),
    labelId: v.id("issueLabels"),
    projectId: v.id("projects"),
    workspaceId: v.id("workspaces"),
  })
    .index("by_issueId", ["issueId"])
    .index("by_issueId_labelId", ["issueId", "labelId"])
    .index("by_labelId", ["labelId"])
    .index("by_projectId", ["projectId"])
    .index("by_workspaceId", ["workspaceId"]),
  issueLabels: defineTable({
    color: v.string(),
    name: v.string(),
    projectId: v.id("projects"),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_workspaceId", ["workspaceId"]),
  issues: defineTable({
    assigneeUserId: v.optional(v.string()),
    archivedAt: v.optional(v.number()),
    completedAt: v.optional(v.number()),
    createdAt: v.optional(v.number()),
    createdByUserId: v.string(),
    description: v.optional(v.string()),
    estimate: v.optional(v.number()),
    identifier: v.string(),
    moduleId: v.optional(v.id("projectModules")),
    parentIssueId: v.optional(v.id("issues")),
    position: v.optional(v.number()),
    priority: issuePriority,
    projectId: v.id("projects"),
    sequenceId: v.number(),
    sprintId: v.optional(v.id("sprints")),
    stateId: v.id("issueStates"),
    startDate: v.optional(v.string()),
    targetDate: v.optional(v.string()),
    title: v.string(),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_projectId_sequenceId", ["projectId", "sequenceId"])
    .index("by_projectId_stateId_position", [
      "projectId",
      "stateId",
      "position",
    ])
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
