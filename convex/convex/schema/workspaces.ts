import { defineTable } from "convex/server";
import { v } from "convex/values";

export const workspaceRole = v.union(
  v.literal("owner"),
  v.literal("admin"),
  v.literal("member"),
  v.literal("guest")
);

export const projectMemberRole = v.union(
  v.literal("admin"),
  v.literal("member")
);

export const projectFeaturesValidator = v.object({
  intake: v.boolean(),
  modules: v.boolean(),
  pages: v.boolean(),
  sprints: v.boolean(),
  views: v.boolean(),
});

export const estimateSystemValidator = v.object({
  enabled: v.boolean(),
  preset: v.union(
    v.literal("linear"),
    v.literal("fibonacci"),
    v.literal("squares"),
    v.literal("custom")
  ),
  values: v.array(v.number()),
});

export const projectAutomationsValidator = v.object({
  autoArchiveClosedMonths: v.union(v.number(), v.null()),
  autoCloseInactiveMonths: v.union(v.number(), v.null()),
});

export const workspaceTables = {
  projectMembers: defineTable({
    projectId: v.id("projects"),
    role: projectMemberRole,
    userId: v.string(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_projectId_userId", ["projectId", "userId"])
    .index("by_userId", ["userId"])
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_userId", ["workspaceId", "userId"]),
  projects: defineTable({
    archivedAt: v.optional(v.number()),
    automations: v.optional(projectAutomationsValidator),
    color: v.string(),
    coverImageUrl: v.optional(v.string()),
    description: v.optional(v.string()),
    estimateSystem: v.optional(estimateSystemValidator),
    features: v.optional(projectFeaturesValidator),
    key: v.string(),
    logoUrl: v.optional(v.string()),
    name: v.string(),
    slug: v.string(),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_key", ["workspaceId", "key"]),
  workspaceInvites: defineTable({
    email: v.string(),
    invitedByUserId: v.string(),
    role: workspaceRole,
    workspaceId: v.id("workspaces"),
  })
    .index("by_email", ["email"])
    .index("by_workspaceId", ["workspaceId"])
    .index("by_workspaceId_email", ["workspaceId", "email"]),
  workspaceMembers: defineTable({
    role: workspaceRole,
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
