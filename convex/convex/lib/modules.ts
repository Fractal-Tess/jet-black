import { ConvexError } from "convex/values";

import type { Doc, Id } from "../_generated/dataModel";
import type { QueryCtx } from "../_generated/server";
import { requireWorkspaceMembership } from "./workspaceAccess";

type DbCtx = Pick<QueryCtx, "db">;

export const MAX_MODULES_PER_PROJECT = 500;
export const MAX_MODULE_ASSIGNMENTS = 2000;
export const MAX_MODULE_LINKS = 100;
export const MAX_MODULE_MEMBERS = 100;

export type ModuleStatus = Doc<"projectModules">["status"];
export type ProgressStateType = Doc<"issueStates">["type"];

export const MODULE_STATUS_LABELS: Record<ModuleStatus, string> = {
  backlog: "Backlog",
  cancelled: "Cancelled",
  completed: "Completed",
  in_progress: "In progress",
  paused: "Paused",
  planned: "Planned",
};

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

export function normalizeModuleName(value: string) {
  const name = value.trim();
  if (!name) {
    throw new ConvexError("Module name is required");
  }
  return name;
}

export function normalizeModuleDate(
  value: string | null | undefined
): string | undefined {
  if (value === null || value === undefined || value.trim() === "") {
    return;
  }

  const date = value.trim();
  if (!DATE_PATTERN.test(date)) {
    throw new ConvexError("Module dates must use YYYY-MM-DD");
  }

  const [yearText, monthText, dayText] = date.split("-");
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const parsed = new Date(Date.UTC(year, month - 1, day));

  if (
    parsed.getUTCFullYear() !== year ||
    parsed.getUTCMonth() + 1 !== month ||
    parsed.getUTCDate() !== day
  ) {
    throw new ConvexError("Module date is invalid");
  }

  return date;
}

export function validateModuleDateRange(
  startDate: string | undefined,
  targetDate: string | undefined
) {
  if (startDate && targetDate && startDate > targetDate) {
    throw new ConvexError("Module start date must not exceed target date");
  }
}

export function normalizeModuleUrl(value: string) {
  const candidate = value.trim();
  let url: URL;

  try {
    url = new URL(candidate);
  } catch {
    throw new ConvexError("Module link URL is invalid");
  }

  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new ConvexError("Module link URL must use http or https");
  }

  return url.toString();
}

export async function requireModuleAccess(
  ctx: DbCtx,
  userId: string,
  moduleId: Id<"projectModules">
) {
  const projectModule = await ctx.db.get(moduleId);
  if (!projectModule) {
    throw new ConvexError("Module not found");
  }

  await requireWorkspaceMembership(ctx, userId, projectModule.workspaceId);
  return projectModule;
}

export async function getAccessibleModule(
  ctx: DbCtx,
  userId: string,
  moduleId: Id<"projectModules">
) {
  const projectModule = await ctx.db.get(moduleId);
  if (!projectModule) {
    return null;
  }

  const membership = await ctx.db
    .query("workspaceMembers")
    .withIndex("by_workspaceId_userId", (q) =>
      q.eq("workspaceId", projectModule.workspaceId).eq("userId", userId)
    )
    .unique();

  return membership ? projectModule : null;
}

export async function validateWorkspaceMemberIds(
  ctx: DbCtx,
  workspaceId: Id<"workspaces">,
  userIds: readonly string[]
) {
  const uniqueUserIds = [...new Set(userIds)];
  if (uniqueUserIds.length > MAX_MODULE_MEMBERS) {
    throw new ConvexError(
      `A module can have at most ${MAX_MODULE_MEMBERS} members`
    );
  }

  for (const userId of uniqueUserIds) {
    if (!userId.trim()) {
      throw new ConvexError("Module member ID is required");
    }

    const membership = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_workspaceId_userId", (q) =>
        q.eq("workspaceId", workspaceId).eq("userId", userId)
      )
      .unique();
    if (!membership) {
      throw new ConvexError("Module members must belong to the workspace");
    }
  }

  return uniqueUserIds;
}

export async function validateWorkspaceLead(
  ctx: DbCtx,
  workspaceId: Id<"workspaces">,
  leadUserId: string | undefined
) {
  if (leadUserId === undefined) {
    return;
  }
  await validateWorkspaceMemberIds(ctx, workspaceId, [leadUserId]);
}

export async function assertUniqueActiveModuleName(
  ctx: DbCtx,
  projectId: Id<"projects">,
  name: string,
  excludedModuleId?: Id<"projectModules">
) {
  const activeModules = await ctx.db
    .query("projectModules")
    .withIndex("by_projectId_and_archivedAt", (q) =>
      q.eq("projectId", projectId).eq("archivedAt", undefined)
    )
    .take(MAX_MODULES_PER_PROJECT + 1);

  if (activeModules.length > MAX_MODULES_PER_PROJECT) {
    throw new ConvexError(
      `A project can have at most ${MAX_MODULES_PER_PROJECT} active modules`
    );
  }

  const normalizedName = name.toLowerCase();
  const duplicate = activeModules.some(
    (projectModule) =>
      projectModule._id !== excludedModuleId &&
      projectModule.name.toLowerCase() === normalizedName
  );
  if (duplicate) {
    throw new ConvexError("An active module with this name already exists");
  }
}

export function groupModuleProgress(stateTypes: readonly ProgressStateType[]) {
  const progress = {
    backlog: 0,
    cancelled: 0,
    completed: 0,
    started: 0,
    total: stateTypes.length,
    unstarted: 0,
  };

  for (const stateType of stateTypes) {
    progress[stateType] += 1;
  }

  return progress;
}
