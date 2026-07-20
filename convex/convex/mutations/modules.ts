import { ConvexError, v } from "convex/values";

import type { Doc, Id } from "../_generated/dataModel";
import type { MutationCtx } from "../_generated/server";
import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import {
  assertUniqueActiveModuleName,
  MAX_MODULE_ASSIGNMENTS,
  MAX_MODULE_LINKS,
  MAX_MODULE_MEMBERS,
  normalizeModuleDate,
  normalizeModuleName,
  normalizeModuleUrl,
  requireModuleAccess,
  validateModuleDateRange,
  validateWorkspaceLead,
  validateWorkspaceMemberIds,
} from "../lib/modules";
import { requireProjectAccess } from "../lib/workspaceAccess";
import { moduleStatusValidator } from "../schema/modules";

type ModuleUpdatePatch = Partial<
  Pick<
    Doc<"projectModules">,
    | "description"
    | "leadUserId"
    | "name"
    | "startDate"
    | "status"
    | "targetDate"
    | "updatedAt"
  >
>;

function requireEditableModule(projectModule: Doc<"projectModules">) {
  if (projectModule.archivedAt !== undefined) {
    throw new ConvexError("Archived modules cannot be edited");
  }
}

async function replaceModuleMembers(
  ctx: MutationCtx,
  projectModule: Doc<"projectModules">,
  userIds: readonly string[]
) {
  const existingMembers = await ctx.db
    .query("projectModuleMembers")
    .withIndex("by_moduleId", (q) => q.eq("moduleId", projectModule._id))
    .take(MAX_MODULE_MEMBERS + 1);
  if (existingMembers.length > MAX_MODULE_MEMBERS) {
    throw new ConvexError("Module member limit exceeded");
  }

  const nextUserIds = new Set(userIds);
  for (const member of existingMembers) {
    if (!nextUserIds.has(member.userId)) {
      await ctx.db.delete(member._id);
    }
    nextUserIds.delete(member.userId);
  }

  for (const userId of nextUserIds) {
    await ctx.db.insert("projectModuleMembers", {
      moduleId: projectModule._id,
      projectId: projectModule.projectId,
      userId,
      workspaceId: projectModule.workspaceId,
    });
  }
}

async function deleteModuleRelations(
  ctx: MutationCtx,
  moduleId: Id<"projectModules">
) {
  const [links, members, assignments, legacyIssues] = await Promise.all([
    ctx.db
      .query("projectModuleLinks")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", moduleId))
      .take(MAX_MODULE_LINKS + 1),
    ctx.db
      .query("projectModuleMembers")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", moduleId))
      .take(MAX_MODULE_MEMBERS + 1),
    ctx.db
      .query("projectModuleIssueAssignments")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", moduleId))
      .take(MAX_MODULE_ASSIGNMENTS + 1),
    ctx.db
      .query("issues")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", moduleId))
      .take(MAX_MODULE_ASSIGNMENTS + 1),
  ]);

  if (
    links.length > MAX_MODULE_LINKS ||
    members.length > MAX_MODULE_MEMBERS ||
    assignments.length > MAX_MODULE_ASSIGNMENTS ||
    legacyIssues.length > MAX_MODULE_ASSIGNMENTS
  ) {
    throw new ConvexError("Module relation limit exceeded");
  }

  for (const relation of [...links, ...members, ...assignments]) {
    await ctx.db.delete(relation._id);
  }
  for (const issue of legacyIssues) {
    await ctx.db.patch(issue._id, {
      moduleId: undefined,
      updatedAt: Date.now(),
    });
  }
}

export const create = mutation({
  args: {
    description: v.optional(v.string()),
    leadUserId: v.optional(v.string()),
    memberIds: v.optional(v.array(v.string())),
    name: v.string(),
    projectId: v.id("projects"),
    startDate: v.optional(v.string()),
    status: v.optional(moduleStatusValidator),
    targetDate: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const name = normalizeModuleName(args.name);
    const startDate = normalizeModuleDate(args.startDate);
    const targetDate = normalizeModuleDate(args.targetDate);
    validateModuleDateRange(startDate, targetDate);
    await assertUniqueActiveModuleName(ctx, project._id, name);
    await validateWorkspaceLead(ctx, project.workspaceId, args.leadUserId);
    const memberIds = await validateWorkspaceMemberIds(
      ctx,
      project.workspaceId,
      args.memberIds ?? []
    );
    const now = Date.now();
    const moduleId = await ctx.db.insert("projectModules", {
      createdAt: now,
      createdByUserId: user._id,
      description: args.description?.trim() || undefined,
      leadUserId: args.leadUserId,
      name,
      projectId: project._id,
      startDate,
      status: args.status ?? "planned",
      targetDate,
      updatedAt: now,
      workspaceId: project.workspaceId,
    });
    const projectModule = await ctx.db.get(moduleId);
    if (!projectModule) {
      throw new ConvexError("Module could not be created");
    }
    await replaceModuleMembers(ctx, projectModule, memberIds);
    return moduleId;
  },
});

export const update = mutation({
  args: {
    description: v.optional(v.union(v.string(), v.null())),
    leadUserId: v.optional(v.union(v.string(), v.null())),
    memberIds: v.optional(v.array(v.string())),
    moduleId: v.id("projectModules"),
    name: v.optional(v.string()),
    startDate: v.optional(v.union(v.string(), v.null())),
    status: v.optional(moduleStatusValidator),
    targetDate: v.optional(v.union(v.string(), v.null())),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      args.moduleId
    );
    requireEditableModule(projectModule);
    const patch: ModuleUpdatePatch = { updatedAt: Date.now() };

    if (args.name !== undefined) {
      const name = normalizeModuleName(args.name);
      await assertUniqueActiveModuleName(
        ctx,
        projectModule.projectId,
        name,
        projectModule._id
      );
      patch.name = name;
    }
    if (args.description !== undefined) {
      patch.description = args.description?.trim() || undefined;
    }
    if (args.status !== undefined) {
      patch.status = args.status;
    }
    if (args.leadUserId !== undefined) {
      const leadUserId = args.leadUserId ?? undefined;
      await validateWorkspaceLead(ctx, projectModule.workspaceId, leadUserId);
      patch.leadUserId = leadUserId;
    }

    const startDate =
      args.startDate === undefined
        ? projectModule.startDate
        : normalizeModuleDate(args.startDate);
    const targetDate =
      args.targetDate === undefined
        ? projectModule.targetDate
        : normalizeModuleDate(args.targetDate);
    validateModuleDateRange(startDate, targetDate);
    if (args.startDate !== undefined) {
      patch.startDate = startDate;
    }
    if (args.targetDate !== undefined) {
      patch.targetDate = targetDate;
    }

    if (args.memberIds !== undefined) {
      const memberIds = await validateWorkspaceMemberIds(
        ctx,
        projectModule.workspaceId,
        args.memberIds
      );
      await replaceModuleMembers(ctx, projectModule, memberIds);
    }

    await ctx.db.patch(projectModule._id, patch);
    return projectModule._id;
  },
});

export const archive = mutation({
  args: { moduleId: v.id("projectModules") },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      args.moduleId
    );
    if (projectModule.archivedAt !== undefined) {
      return projectModule._id;
    }
    if (
      projectModule.status !== "completed" &&
      projectModule.status !== "cancelled"
    ) {
      throw new ConvexError(
        "Only completed or cancelled modules can be archived"
      );
    }

    await ctx.db.patch(projectModule._id, {
      archivedAt: Date.now(),
      updatedAt: Date.now(),
    });
    return projectModule._id;
  },
});

export const restore = mutation({
  args: { moduleId: v.id("projectModules") },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      args.moduleId
    );
    if (projectModule.archivedAt === undefined) {
      return projectModule._id;
    }

    await assertUniqueActiveModuleName(
      ctx,
      projectModule.projectId,
      projectModule.name,
      projectModule._id
    );
    await ctx.db.patch(projectModule._id, {
      archivedAt: undefined,
      updatedAt: Date.now(),
    });
    return projectModule._id;
  },
});

export const remove = mutation({
  args: { moduleId: v.id("projectModules") },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      args.moduleId
    );
    await deleteModuleRelations(ctx, projectModule._id);
    await ctx.db.delete(projectModule._id);
    return projectModule._id;
  },
});

export const addLink = mutation({
  args: {
    moduleId: v.id("projectModules"),
    title: v.string(),
    url: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      args.moduleId
    );
    requireEditableModule(projectModule);
    const title = args.title.trim();
    if (!title) {
      throw new ConvexError("Module link title is required");
    }
    const url = normalizeModuleUrl(args.url);
    const existingLink = await ctx.db
      .query("projectModuleLinks")
      .withIndex("by_moduleId_and_url", (q) =>
        q.eq("moduleId", projectModule._id).eq("url", url)
      )
      .unique();
    if (existingLink) {
      return existingLink._id;
    }
    const links = await ctx.db
      .query("projectModuleLinks")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", projectModule._id))
      .take(MAX_MODULE_LINKS);
    if (links.length >= MAX_MODULE_LINKS) {
      throw new ConvexError(
        `A module can have at most ${MAX_MODULE_LINKS} links`
      );
    }

    const now = Date.now();
    return await ctx.db.insert("projectModuleLinks", {
      createdAt: now,
      createdByUserId: user._id,
      moduleId: projectModule._id,
      projectId: projectModule.projectId,
      title,
      updatedAt: now,
      url,
      workspaceId: projectModule.workspaceId,
    });
  },
});

export const updateLink = mutation({
  args: {
    linkId: v.id("projectModuleLinks"),
    title: v.optional(v.string()),
    url: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const link = await ctx.db.get(args.linkId);
    if (!link) {
      throw new ConvexError("Module link not found");
    }
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      link.moduleId
    );
    requireEditableModule(projectModule);
    const patch: Partial<
      Pick<Doc<"projectModuleLinks">, "title" | "updatedAt" | "url">
    > = {
      updatedAt: Date.now(),
    };

    if (args.title !== undefined) {
      const title = args.title.trim();
      if (!title) {
        throw new ConvexError("Module link title is required");
      }
      patch.title = title;
    }
    if (args.url !== undefined) {
      const url = normalizeModuleUrl(args.url);
      const duplicate = await ctx.db
        .query("projectModuleLinks")
        .withIndex("by_moduleId_and_url", (q) =>
          q.eq("moduleId", projectModule._id).eq("url", url)
        )
        .unique();
      if (duplicate && duplicate._id !== link._id) {
        throw new ConvexError("This link is already attached to the module");
      }
      patch.url = url;
    }

    await ctx.db.patch(link._id, patch);
    return link._id;
  },
});

export const removeLink = mutation({
  args: { linkId: v.id("projectModuleLinks") },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const link = await ctx.db.get(args.linkId);
    if (!link) {
      throw new ConvexError("Module link not found");
    }
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      link.moduleId
    );
    requireEditableModule(projectModule);
    await ctx.db.delete(link._id);
    return link._id;
  },
});

export const addIssueAssignment = mutation({
  args: {
    issueId: v.id("issues"),
    moduleId: v.id("projectModules"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      args.moduleId
    );
    requireEditableModule(projectModule);
    const issue = await ctx.db.get(args.issueId);
    if (
      !issue ||
      issue.projectId !== projectModule.projectId ||
      issue.workspaceId !== projectModule.workspaceId
    ) {
      throw new ConvexError("Issue and module must belong to the same project");
    }
    const existingAssignment = await ctx.db
      .query("projectModuleIssueAssignments")
      .withIndex("by_moduleId_and_issueId", (q) =>
        q.eq("moduleId", projectModule._id).eq("issueId", issue._id)
      )
      .unique();
    if (existingAssignment) {
      return existingAssignment._id;
    }
    const assignments = await ctx.db
      .query("projectModuleIssueAssignments")
      .withIndex("by_moduleId", (q) => q.eq("moduleId", projectModule._id))
      .take(MAX_MODULE_ASSIGNMENTS);
    if (assignments.length >= MAX_MODULE_ASSIGNMENTS) {
      throw new ConvexError(
        `A module can have at most ${MAX_MODULE_ASSIGNMENTS} issue assignments`
      );
    }

    const assignmentId = await ctx.db.insert("projectModuleIssueAssignments", {
      assignedAt: Date.now(),
      assignedByUserId: user._id,
      issueId: issue._id,
      moduleId: projectModule._id,
      projectId: projectModule.projectId,
      workspaceId: projectModule.workspaceId,
    });
    if (issue.moduleId === undefined) {
      await ctx.db.patch(issue._id, {
        moduleId: projectModule._id,
        updatedAt: Date.now(),
      });
    }
    return assignmentId;
  },
});

export const removeIssueAssignment = mutation({
  args: {
    issueId: v.id("issues"),
    moduleId: v.id("projectModules"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const projectModule = await requireModuleAccess(
      ctx,
      user._id,
      args.moduleId
    );
    requireEditableModule(projectModule);
    const issue = await ctx.db.get(args.issueId);
    if (
      !issue ||
      issue.projectId !== projectModule.projectId ||
      issue.workspaceId !== projectModule.workspaceId
    ) {
      throw new ConvexError("Issue and module must belong to the same project");
    }
    const assignment = await ctx.db
      .query("projectModuleIssueAssignments")
      .withIndex("by_moduleId_and_issueId", (q) =>
        q.eq("moduleId", projectModule._id).eq("issueId", issue._id)
      )
      .unique();
    if (assignment) {
      await ctx.db.delete(assignment._id);
    }
    if (issue.moduleId === projectModule._id) {
      await ctx.db.patch(issue._id, {
        moduleId: undefined,
        updatedAt: Date.now(),
      });
    }
    return {
      removed: assignment !== null || issue.moduleId === projectModule._id,
    };
  },
});
