import { defineTable } from "convex/server";
import { v } from "convex/values";

export const pageTables = {
  projectPages: defineTable({
    content: v.string(),
    createdAt: v.number(),
    createdByUserId: v.string(),
    icon: v.optional(v.string()),
    projectId: v.id("projects"),
    title: v.string(),
    updatedAt: v.number(),
    workspaceId: v.id("workspaces"),
  })
    .index("by_projectId", ["projectId"])
    .index("by_workspaceId", ["workspaceId"])
    .searchIndex("search_title", {
      filterFields: ["projectId", "workspaceId"],
      searchField: "title",
    }),
};
