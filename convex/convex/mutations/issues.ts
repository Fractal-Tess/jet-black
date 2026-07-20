import { ConvexError, v } from "convex/values";

import type { Doc, Id } from "../_generated/dataModel";
import type { MutationCtx } from "../_generated/server";
import { mutation } from "../_generated/server";
import { authComponent } from "../auth";
import { requireAuthUser } from "../lib/auth";
import { getDefaultStateId, getNextSequenceId } from "../lib/issues";
import { MAX_MODULE_ASSIGNMENTS } from "../lib/modules";
import {
  requireIssueAccess,
  requireProjectAccess,
  requireWorkspaceMembership,
} from "../lib/workspaceAccess";

const priorityValidator = v.union(
  v.literal("none"),
  v.literal("low"),
  v.literal("medium"),
  v.literal("high"),
  v.literal("urgent")
);

type IssueUpdatePatch = Partial<{
  assigneeUserId: string | undefined;
  completedAt: number | undefined;
  createdByUserId: string;
  description: string | undefined;
  estimate: number | undefined;
  moduleId: Id<"projectModules"> | undefined;
  priority: "none" | "low" | "medium" | "high" | "urgent";
  sprintId: Id<"sprints"> | undefined;
  startDate: string | undefined;
  stateId: Id<"issueStates">;
  targetDate: string | undefined;
  title: string;
  updatedAt: number;
}>;

type IssueUpdateArgs = {
  assigneeUserId?: string | null;
  createdByUserId?: string;
  description?: string;
  estimate?: number | null;
  issueId: Id<"issues">;
  moduleId?: Id<"projectModules"> | null;
  priority?: "none" | "low" | "medium" | "high" | "urgent";
  sprintId?: Id<"sprints"> | null;
  startDate?: string | null;
  stateId?: Id<"issueStates">;
  targetDate?: string | null;
  title?: string;
};

function applyScalarIssuePatch(args: IssueUpdateArgs, patch: IssueUpdatePatch) {
  if (args.assigneeUserId !== undefined) {
    patch.assigneeUserId = args.assigneeUserId ?? undefined;
  }

  if (args.createdByUserId !== undefined) {
    patch.createdByUserId = args.createdByUserId;
  }

  if (args.title !== undefined) {
    const title = args.title.trim();
    if (!title) {
      throw new ConvexError("Issue title is required");
    }
    patch.title = title;
  }

  if (args.description !== undefined) {
    patch.description = args.description.trim() || undefined;
  }

  if (args.priority !== undefined) {
    patch.priority = args.priority;
  }

  if (args.estimate !== undefined) {
    patch.estimate = args.estimate ?? undefined;
  }

  if (args.startDate !== undefined) {
    patch.startDate = args.startDate?.trim() || undefined;
  }

  if (args.targetDate !== undefined) {
    patch.targetDate = args.targetDate?.trim() || undefined;
  }
}

async function applySprintPatch(
  ctx: MutationCtx,
  args: IssueUpdateArgs,
  issue: Doc<"issues">,
  patch: IssueUpdatePatch
) {
  if (args.sprintId === undefined) {
    return;
  }

  const sprint = args.sprintId ? await ctx.db.get(args.sprintId) : null;

  if (args.sprintId && sprint?.projectId !== issue.projectId) {
    throw new ConvexError("Invalid sprint");
  }

  patch.sprintId = args.sprintId ?? undefined;
}

async function applyModulePatch(
  ctx: MutationCtx,
  args: IssueUpdateArgs,
  issue: Doc<"issues">,
  patch: IssueUpdatePatch
) {
  if (args.moduleId === undefined) {
    return;
  }

  const projectModule = args.moduleId ? await ctx.db.get(args.moduleId) : null;

  if (
    args.moduleId &&
    (!projectModule ||
      projectModule.projectId !== issue.projectId ||
      projectModule.workspaceId !== issue.workspaceId)
  ) {
    throw new ConvexError("Invalid module");
  }

  if (projectModule?.archivedAt !== undefined) {
    throw new ConvexError("Archived modules cannot receive assignments");
  }

  patch.moduleId = args.moduleId ?? undefined;
}

async function synchronizePrimaryModuleAssignment(
  ctx: MutationCtx,
  issue: Doc<"issues">,
  nextModuleId: Id<"projectModules"> | undefined,
  actorUserId: string
) {
  const previousModuleId = issue.moduleId;
  if (previousModuleId && previousModuleId !== nextModuleId) {
    const previousAssignment = await ctx.db
      .query("projectModuleIssueAssignments")
      .withIndex("by_moduleId_and_issueId", (q) =>
        q.eq("moduleId", previousModuleId).eq("issueId", issue._id)
      )
      .unique();
    if (previousAssignment) {
      await ctx.db.delete(previousAssignment._id);
    }
  }

  if (!nextModuleId) {
    return;
  }

  const existingAssignment = await ctx.db
    .query("projectModuleIssueAssignments")
    .withIndex("by_moduleId_and_issueId", (q) =>
      q.eq("moduleId", nextModuleId).eq("issueId", issue._id)
    )
    .unique();
  if (existingAssignment) {
    return;
  }

  const assignments = await ctx.db
    .query("projectModuleIssueAssignments")
    .withIndex("by_moduleId", (q) => q.eq("moduleId", nextModuleId))
    .take(MAX_MODULE_ASSIGNMENTS);
  if (assignments.length >= MAX_MODULE_ASSIGNMENTS) {
    throw new ConvexError(
      `A module can have at most ${MAX_MODULE_ASSIGNMENTS} issue assignments`
    );
  }

  await ctx.db.insert("projectModuleIssueAssignments", {
    assignedAt: Date.now(),
    assignedByUserId: actorUserId,
    issueId: issue._id,
    moduleId: nextModuleId,
    projectId: issue.projectId,
    workspaceId: issue.workspaceId,
  });
}

async function applyStatePatch(
  ctx: MutationCtx,
  args: IssueUpdateArgs,
  issue: Doc<"issues">,
  patch: IssueUpdatePatch
) {
  if (args.stateId === undefined) {
    return;
  }

  const state = await ctx.db.get(args.stateId);

  if (!state || state.projectId !== issue.projectId) {
    throw new ConvexError("Invalid issue state");
  }

  patch.stateId = args.stateId;
  patch.completedAt =
    state.type === "completed" ? (issue.completedAt ?? Date.now()) : undefined;
}

export const create = mutation({
  args: {
    assigneeUserId: v.optional(v.string()),
    description: v.optional(v.string()),
    estimate: v.optional(v.number()),
    moduleId: v.optional(v.id("projectModules")),
    parentIssueId: v.optional(v.id("issues")),
    priority: priorityValidator,
    projectId: v.id("projects"),
    sprintId: v.optional(v.id("sprints")),
    startDate: v.optional(v.string()),
    stateId: v.optional(v.id("issueStates")),
    targetDate: v.optional(v.string()),
    title: v.string(),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const title = args.title.trim();

    if (!title) {
      throw new ConvexError("Issue title is required");
    }

    const now = Date.now();
    const sequenceId = await getNextSequenceId(ctx, project._id);
    const stateId = args.stateId ?? (await getDefaultStateId(ctx, project._id));
    const state = await ctx.db.get(stateId);
    const parentIssue = args.parentIssueId
      ? await ctx.db.get(args.parentIssueId)
      : null;
    const projectModule = args.moduleId
      ? await ctx.db.get(args.moduleId)
      : null;
    const sprint = args.sprintId ? await ctx.db.get(args.sprintId) : null;

    if (!state || state.projectId !== project._id) {
      throw new ConvexError("Invalid issue state");
    }

    if (args.parentIssueId && parentIssue?.projectId !== project._id) {
      throw new ConvexError("Invalid parent issue");
    }

    if (
      args.moduleId &&
      (!projectModule ||
        projectModule.projectId !== project._id ||
        projectModule.workspaceId !== project.workspaceId)
    ) {
      throw new ConvexError("Invalid module");
    }

    if (projectModule?.archivedAt !== undefined) {
      throw new ConvexError("Archived modules cannot receive assignments");
    }

    if (args.sprintId && sprint?.projectId !== project._id) {
      throw new ConvexError("Invalid sprint");
    }

    const issueId = await ctx.db.insert("issues", {
      assigneeUserId: args.assigneeUserId,
      createdByUserId: user._id,
      createdAt: now,
      description: args.description?.trim() || undefined,
      estimate: args.estimate,
      identifier: `${project.key}-${sequenceId}`,
      moduleId: args.moduleId,
      parentIssueId: args.parentIssueId,
      position: now,
      priority: args.priority,
      projectId: project._id,
      sequenceId,
      sprintId: args.sprintId,
      startDate: args.startDate,
      stateId,
      targetDate: args.targetDate,
      title,
      updatedAt: now,
      workspaceId: project.workspaceId,
    });

    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId,
      message: args.parentIssueId ? "created a sub-issue" : "created the issue",
      workspaceId: project.workspaceId,
    });

    if (args.moduleId) {
      const issue = await ctx.db.get(issueId);
      if (!issue) {
        throw new ConvexError("Issue could not be created");
      }
      await synchronizePrimaryModuleAssignment(
        ctx,
        issue,
        args.moduleId,
        user._id
      );
    }

    return issueId;
  },
});

function buildDateActivityMessage(
  nextValue: string | null | undefined,
  currentValue: string | undefined,
  dateLabel: string
): string | null {
  if (nextValue === undefined) {
    return null;
  }
  const normalized = nextValue?.trim() || undefined;
  if (normalized === currentValue) {
    return null;
  }
  return normalized
    ? `set the ${dateLabel} to ${normalized}`
    : `removed the ${dateLabel}`;
}

function buildScalarActivityMessages(
  args: IssueUpdateArgs,
  issue: Doc<"issues">
): string[] {
  const messages: string[] = [];

  if (args.title !== undefined) {
    const title = args.title.trim();
    if (title && title !== issue.title) {
      messages.push(`renamed the issue to "${title}"`);
    }
  }

  if (
    args.description !== undefined &&
    (args.description.trim() || undefined) !== issue.description
  ) {
    messages.push("updated the description");
  }

  if (args.priority !== undefined && args.priority !== issue.priority) {
    messages.push(`set priority to ${args.priority}`);
  }

  const startDateMessage = buildDateActivityMessage(
    args.startDate,
    issue.startDate,
    "start date"
  );
  if (startDateMessage) {
    messages.push(startDateMessage);
  }

  const targetDateMessage = buildDateActivityMessage(
    args.targetDate,
    issue.targetDate,
    "due date"
  );
  if (targetDateMessage) {
    messages.push(targetDateMessage);
  }

  return messages;
}

async function buildRelationActivityMessages(
  ctx: MutationCtx,
  args: IssueUpdateArgs,
  issue: Doc<"issues">
): Promise<string[]> {
  const messages: string[] = [];

  if (args.stateId !== undefined && args.stateId !== issue.stateId) {
    const state = await ctx.db.get(args.stateId);
    messages.push(`changed the state to ${state?.name ?? "an unknown state"}`);
  }

  if (
    args.assigneeUserId !== undefined &&
    (args.assigneeUserId ?? undefined) !== issue.assigneeUserId
  ) {
    if (args.assigneeUserId) {
      const assignee = await authComponent.getAnyUserById(
        ctx,
        args.assigneeUserId
      );
      messages.push(`assigned ${assignee?.name ?? "a member"}`);
    } else {
      messages.push("removed the assignee");
    }
  }

  if (
    args.moduleId !== undefined &&
    (args.moduleId ?? undefined) !== issue.moduleId
  ) {
    if (args.moduleId) {
      const projectModule = await ctx.db.get(args.moduleId);
      messages.push(
        `moved to module ${projectModule?.name ?? "an unknown module"}`
      );
    } else {
      messages.push("removed the issue from its module");
    }
  }

  return messages;
}

export const update = mutation({
  args: {
    assigneeUserId: v.optional(v.union(v.string(), v.null())),
    createdByUserId: v.optional(v.string()),
    description: v.optional(v.string()),
    estimate: v.optional(v.union(v.number(), v.null())),
    issueId: v.id("issues"),
    moduleId: v.optional(v.union(v.id("projectModules"), v.null())),
    priority: v.optional(priorityValidator),
    sprintId: v.optional(v.union(v.id("sprints"), v.null())),
    stateId: v.optional(v.id("issueStates")),
    startDate: v.optional(v.union(v.string(), v.null())),
    targetDate: v.optional(v.union(v.string(), v.null())),
    title: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const patch: IssueUpdatePatch = { updatedAt: Date.now() };

    // The reporter must be a member of the issue's workspace.
    if (args.createdByUserId !== undefined) {
      await requireWorkspaceMembership(
        ctx,
        args.createdByUserId,
        issue.workspaceId
      );
    }

    applyScalarIssuePatch(args, patch);
    await applyModulePatch(ctx, args, issue, patch);
    await applySprintPatch(ctx, args, issue, patch);
    await applyStatePatch(ctx, args, issue, patch);

    await ctx.db.patch(args.issueId, patch);
    if (args.moduleId !== undefined) {
      await synchronizePrimaryModuleAssignment(
        ctx,
        issue,
        args.moduleId ?? undefined,
        user._id
      );
    }
    const activityMessages = [
      ...buildScalarActivityMessages(args, issue),
      ...(await buildRelationActivityMessages(ctx, args, issue)),
    ];

    await Promise.all(
      activityMessages.map((message) =>
        ctx.db.insert("issueActivities", {
          actorUserId: user._id,
          issueId: args.issueId,
          message,
          workspaceId: issue.workspaceId,
        })
      )
    );

    return args.issueId;
  },
});

export const move = mutation({
  args: {
    issueId: v.id("issues"),
    position: v.number(),
    stateId: v.id("issueStates"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);
    const state = await ctx.db.get(args.stateId);

    if (!state || state.projectId !== issue.projectId) {
      throw new ConvexError("Invalid issue state");
    }

    await ctx.db.patch(args.issueId, {
      completedAt:
        state.type === "completed"
          ? (issue.completedAt ?? Date.now())
          : undefined,
      position: args.position,
      stateId: args.stateId,
      updatedAt: Date.now(),
    });
    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: args.issueId,
      message:
        args.stateId === issue.stateId
          ? "reordered the issue"
          : "moved the issue",
      workspaceId: issue.workspaceId,
    });

    return args.issueId;
  },
});

export const archive = mutation({
  args: {
    issueId: v.id("issues"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const issue = await requireIssueAccess(ctx, user._id, args.issueId);

    if (issue.archivedAt) {
      return args.issueId;
    }

    await ctx.db.patch(args.issueId, {
      archivedAt: Date.now(),
      updatedAt: Date.now(),
    });
    await ctx.db.insert("issueActivities", {
      actorUserId: user._id,
      issueId: args.issueId,
      message: "archived the issue",
      workspaceId: issue.workspaceId,
    });

    return args.issueId;
  },
});
