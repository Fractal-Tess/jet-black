import { v } from "convex/values";

import type { Doc, Id } from "../_generated/dataModel";
import type { QueryCtx } from "../_generated/server";
import { query } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  getAccessibleModule,
  groupModuleProgress,
  MAX_MODULE_ASSIGNMENTS,
  MAX_MODULE_LINKS,
  MAX_MODULE_MEMBERS,
  MAX_MODULES_PER_PROJECT,
} from "../lib/modules";
import { requireProjectAccess } from "../lib/workspaceAccess";

async function issuesForModule(
  ctx: QueryCtx,
  projectModule: Doc<"projectModules">
) {
  const [assignments, legacyIssues] = await Promise.all([
    ctx.db
      .query("projectModuleIssueAssignments")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", projectModule._id))
      .take(MAX_MODULE_ASSIGNMENTS),
    ctx.db
      .query("issues")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", projectModule._id))
      .take(MAX_MODULE_ASSIGNMENTS),
  ]);
  const issuesById = new Map<Id<"issues">, Doc<"issues">>();

  for (const issue of legacyIssues) {
    if (
      issue.projectId === projectModule.projectId &&
      issue.workspaceId === projectModule.workspaceId
    ) {
      issuesById.set(issue._id, issue);
    }
  }
  for (const assignment of assignments) {
    const issue = await ctx.db.get(assignment.issueId);
    if (
      issue &&
      issue.projectId === projectModule.projectId &&
      issue.workspaceId === projectModule.workspaceId
    ) {
      issuesById.set(issue._id, issue);
    }
  }

  return [...issuesById.values()];
}

async function enrichModule(
  ctx: QueryCtx,
  projectModule: Doc<"projectModules">
) {
  const [links, members, issues] = await Promise.all([
    ctx.db
      .query("projectModuleLinks")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", projectModule._id))
      .take(MAX_MODULE_LINKS),
    ctx.db
      .query("projectModuleMembers")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", projectModule._id))
      .take(MAX_MODULE_MEMBERS),
    issuesForModule(ctx, projectModule),
  ]);
  const stateTypes: Doc<"issueStates">["type"][] = [];

  for (const issue of issues) {
    const state = await ctx.db.get(issue.stateId);
    if (state && state.projectId === projectModule.projectId) {
      stateTypes.push(state.type);
    }
  }

  return {
    ...projectModule,
    issueIds: issues.map((issue) => issue._id),
    links,
    memberIds: members.map((member) => member.userId),
    progress: groupModuleProgress(stateTypes),
  };
}

async function enrichModules(
  ctx: QueryCtx,
  projectModules: readonly Doc<"projectModules">[]
) {
  return await Promise.all(
    projectModules.map(
      async (projectModule) => await enrichModule(ctx, projectModule)
    )
  );
}

export const listForProject = query({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireProjectAccess(ctx, user._id, args.projectId);
    const projectModules = await ctx.db
      .query("projectModules")
      .withIndex("by_projectId_and_archivedAt", (q) =>
        q.eq("projectId", args.projectId).eq("archivedAt", undefined)
      )
      .order("desc")
      .take(MAX_MODULES_PER_PROJECT);

    return await enrichModules(ctx, projectModules);
  },
});

export const get = query({
  args: {
    moduleId: v.id("projectModules"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await getAccessibleModule(
      ctx,
      user._id,
      args.moduleId
    );
    if (!projectModule) {
      return null;
    }

    return await enrichModule(ctx, projectModule);
  },
});

export const listArchivedForProject = query({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireProjectAccess(ctx, user._id, args.projectId);
    const projectModules = await ctx.db
      .query("projectModules")
      .withIndex("by_projectId_and_archivedAt", (q) =>
        q.eq("projectId", args.projectId).gt("archivedAt", undefined)
      )
      .order("desc")
      .take(MAX_MODULES_PER_PROJECT);

    return await enrichModules(ctx, projectModules);
  },
});
