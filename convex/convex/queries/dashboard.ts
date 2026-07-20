import { ConvexError, v } from "convex/values";

import type { Doc, Id } from "../_generated/dataModel";
import type { QueryCtx } from "../_generated/server";
import { query } from "../_generated/server";
import { authComponent } from "../auth";
import { requireAuthUser } from "../lib/auth";
import { requireWorkspaceMembership } from "../lib/workspaceAccess";

const RECENT_ACTIVITY_LIMIT = 20;
const RECENT_ISSUE_LIMIT = 8;
const MY_ISSUE_LIMIT = 8;
const DAY_MS = 24 * 60 * 60 * 1000;
const WEEK_MS = 7 * DAY_MS;
const TREND_DAYS = 14;

type IssueStateType = Doc<"issueStates">["type"];
type SelectedProject = Doc<"projects"> | null;

type AnalyticsScope = {
  selectedProject: SelectedProject;
  workspaceId: Id<"workspaces">;
};

type ProjectAnalytics = {
  cancelledIssues: number;
  color: string | null;
  completedIssues: number;
  name: string;
  openIssues: number;
  projectId: Id<"projects">;
  startedIssues: number;
  totalIssues: number;
};

type TrendBucket = {
  completed: number;
  created: number;
  date: number;
};

type IssueAnalytics = {
  activityStart: number;
  completedDurations: number[];
  completedInPeriod: number;
  createdInPeriod: number;
  overdueIssues: number;
  projectAnalytics: Map<Id<"projects">, ProjectAnalytics>;
  statusCounts: Record<IssueStateType, number>;
  todayStart: number;
  totalIssues: number;
  trend: TrendBucket[];
  trendStart: number;
};

const createProjectAnalytics = (
  project: Doc<"projects">
): ProjectAnalytics => ({
  cancelledIssues: 0,
  color: project.color ?? null,
  completedIssues: 0,
  name: project.name,
  openIssues: 0,
  projectId: project._id,
  startedIssues: 0,
  totalIssues: 0,
});

const startOfUtcDay = (timestamp: number) => {
  const date = new Date(timestamp);
  return Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate());
};

const targetDateTimestamp = (targetDate: string) =>
  new Date(`${targetDate}T00:00:00.000Z`).getTime();

const getSelectedProject = async (
  ctx: QueryCtx,
  workspaceId: Id<"workspaces">,
  projectId?: Id<"projects">
): Promise<SelectedProject> => {
  if (!projectId) {
    return null;
  }

  const project = await ctx.db.get(projectId);
  if (!project) {
    throw new ConvexError("Project not found");
  }

  if (project.workspaceId !== workspaceId) {
    throw new ConvexError("Project does not belong to this workspace");
  }

  return project;
};

const listScopedProjects = async (
  ctx: QueryCtx,
  scope: AnalyticsScope
): Promise<Doc<"projects">[]> => {
  if (scope.selectedProject) {
    return [scope.selectedProject];
  }

  const projects: Doc<"projects">[] = [];
  const projectQuery = ctx.db
    .query("projects")
    .withIndex("by_workspaceId", (q) => q.eq("workspaceId", scope.workspaceId));

  for await (const project of projectQuery) {
    projects.push(project);
  }

  return projects;
};

const loadScopedStateTypes = async (
  ctx: QueryCtx,
  scope: AnalyticsScope
): Promise<Map<Id<"issueStates">, IssueStateType>> => {
  const stateById = new Map<Id<"issueStates">, IssueStateType>();
  const projectId = scope.selectedProject?._id;
  const stateQuery = projectId
    ? ctx.db
        .query("issueStates")
        .withIndex("by_projectId", (q) => q.eq("projectId", projectId))
    : ctx.db
        .query("issueStates")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", scope.workspaceId)
        );

  for await (const state of stateQuery) {
    stateById.set(state._id, state.type);
  }

  return stateById;
};

const createIssueAnalytics = (
  projects: Doc<"projects">[],
  now: number
): IssueAnalytics => {
  const trendStart = startOfUtcDay(now) - (TREND_DAYS - 1) * DAY_MS;

  return {
    activityStart: now - WEEK_MS,
    completedDurations: [],
    completedInPeriod: 0,
    createdInPeriod: 0,
    overdueIssues: 0,
    projectAnalytics: new Map(
      projects.map((project) => [project._id, createProjectAnalytics(project)])
    ),
    statusCounts: {
      backlog: 0,
      cancelled: 0,
      completed: 0,
      started: 0,
      unstarted: 0,
    },
    todayStart: startOfUtcDay(now),
    totalIssues: 0,
    trend: Array.from({ length: TREND_DAYS }, (_, index) => ({
      completed: 0,
      created: 0,
      date: trendStart + index * DAY_MS,
    })),
    trendStart,
  };
};

const recordIssueHistory = (
  issue: Doc<"issues">,
  analytics: IssueAnalytics
) => {
  const createdAt = issue.createdAt ?? issue._creationTime;
  const createdTrendIndex = Math.floor(
    (createdAt - analytics.trendStart) / DAY_MS
  );
  const createdBucket = analytics.trend[createdTrendIndex];

  if (createdBucket) {
    createdBucket.created += 1;
  }

  if (createdAt >= analytics.activityStart) {
    analytics.createdInPeriod += 1;
  }

  if (!issue.completedAt) {
    return;
  }

  const completedTrendIndex = Math.floor(
    (issue.completedAt - analytics.trendStart) / DAY_MS
  );
  const completedBucket = analytics.trend[completedTrendIndex];

  if (completedBucket) {
    completedBucket.completed += 1;
  }

  const duration = issue.completedAt - createdAt;
  if (duration >= 0) {
    analytics.completedDurations.push(duration);
  }

  if (issue.completedAt >= analytics.activityStart) {
    analytics.completedInPeriod += 1;
  }
};

const recordCurrentIssue = (
  issue: Doc<"issues">,
  stateType: IssueStateType,
  analytics: IssueAnalytics
) => {
  analytics.totalIssues += 1;
  analytics.statusCounts[stateType] += 1;
  const project = analytics.projectAnalytics.get(issue.projectId);

  if (project) {
    project.totalIssues += 1;
  }

  if (stateType === "completed") {
    if (project) {
      project.completedIssues += 1;
    }
    return;
  }

  if (stateType === "cancelled") {
    if (project) {
      project.cancelledIssues += 1;
    }
    return;
  }

  if (project) {
    project.openIssues += 1;
  }

  if (stateType === "started" && project) {
    project.startedIssues += 1;
  }

  if (
    issue.targetDate &&
    targetDateTimestamp(issue.targetDate) < analytics.todayStart
  ) {
    analytics.overdueIssues += 1;
  }
};

const aggregateScopedIssues = async (
  ctx: QueryCtx,
  scope: AnalyticsScope,
  projects: Doc<"projects">[],
  stateById: Map<Id<"issueStates">, IssueStateType>,
  now: number
): Promise<IssueAnalytics> => {
  const analytics = createIssueAnalytics(projects, now);
  const projectId = scope.selectedProject?._id;
  const issueQuery = projectId
    ? ctx.db
        .query("issues")
        .withIndex("by_projectId", (q) => q.eq("projectId", projectId))
    : ctx.db
        .query("issues")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", scope.workspaceId)
        );

  for await (const issue of issueQuery) {
    recordIssueHistory(issue, analytics);

    if (!issue.archivedAt) {
      recordCurrentIssue(
        issue,
        stateById.get(issue.stateId) ?? "unstarted",
        analytics
      );
    }
  }

  return analytics;
};

const countRows = async <Row>(
  rows: AsyncIterable<Row>,
  shouldCount: (row: Row) => boolean = () => true
) => {
  let count = 0;

  for await (const row of rows) {
    if (shouldCount(row)) {
      count += 1;
    }
  }

  return count;
};

const countScopedCycles = async (ctx: QueryCtx, scope: AnalyticsScope) => {
  const projectId = scope.selectedProject?._id;
  const rows = projectId
    ? ctx.db
        .query("sprints")
        .withIndex("by_projectId", (q) => q.eq("projectId", projectId))
    : ctx.db
        .query("sprints")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", scope.workspaceId)
        );

  return await countRows(rows);
};

const countScopedIntake = async (ctx: QueryCtx, scope: AnalyticsScope) => {
  const projectId = scope.selectedProject?._id;
  const rows = projectId
    ? ctx.db
        .query("intakeIssues")
        .withIndex("by_projectId", (q) => q.eq("projectId", projectId))
    : ctx.db
        .query("intakeIssues")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", scope.workspaceId)
        );

  return await countRows(rows);
};

const countScopedModules = async (ctx: QueryCtx, scope: AnalyticsScope) => {
  const projectId = scope.selectedProject?._id;
  const rows = projectId
    ? ctx.db
        .query("projectModules")
        .withIndex("by_projectId", (q) => q.eq("projectId", projectId))
    : ctx.db
        .query("projectModules")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", scope.workspaceId)
        );

  return await countRows(rows, (module) => !module.archivedAt);
};

const countScopedPages = async (ctx: QueryCtx, scope: AnalyticsScope) => {
  const projectId = scope.selectedProject?._id;
  const rows = projectId
    ? ctx.db
        .query("projectPages")
        .withIndex("by_projectId", (q) => q.eq("projectId", projectId))
    : ctx.db
        .query("projectPages")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", scope.workspaceId)
        );

  return await countRows(rows);
};

const countWorkspacePeople = async (
  ctx: QueryCtx,
  workspaceId: Id<"workspaces">
) => {
  let owners = 0;
  let members = 0;
  const membershipQuery = ctx.db
    .query("workspaceMembers")
    .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId));

  for await (const membership of membershipQuery) {
    if (membership.role === "owner") {
      owners += 1;
    } else {
      members += 1;
    }
  }

  return { members, owners, total: members + owners };
};

const getScopedTotals = async (ctx: QueryCtx, scope: AnalyticsScope) => {
  const [cycles, intake, modules, pages] = await Promise.all([
    countScopedCycles(ctx, scope),
    countScopedIntake(ctx, scope),
    countScopedModules(ctx, scope),
    countScopedPages(ctx, scope),
  ]);

  return { cycles, intake, modules, pages };
};

const averageDuration = (durations: number[]) => {
  if (durations.length === 0) {
    return null;
  }

  return Math.round(
    durations.reduce((sum, value) => sum + value, 0) / durations.length
  );
};

const summarizeProjects = (
  projectAnalytics: Map<Id<"projects">, ProjectAnalytics>
) =>
  [...projectAnalytics.values()]
    .map((project) => ({
      ...project,
      progress:
        project.totalIssues === 0
          ? 0
          : Math.round((project.completedIssues / project.totalIssues) * 100),
    }))
    .sort((left, right) => {
      if (right.totalIssues === left.totalIssues) {
        return left.name.localeCompare(right.name);
      }

      return right.totalIssues - left.totalIssues;
    });

const buildWorkspaceAnalytics = async (
  ctx: QueryCtx,
  scope: AnalyticsScope
) => {
  const projects = await listScopedProjects(ctx, scope);
  const stateById = await loadScopedStateTypes(ctx, scope);
  const [issueAnalytics, people, scopedTotals] = await Promise.all([
    aggregateScopedIssues(ctx, scope, projects, stateById, Date.now()),
    countWorkspacePeople(ctx, scope.workspaceId),
    getScopedTotals(ctx, scope),
  ]);

  return {
    people,
    projects: summarizeProjects(issueAnalytics.projectAnalytics),
    scope: {
      projectId: scope.selectedProject?._id ?? null,
      projectName: scope.selectedProject?.name ?? null,
    },
    stats: {
      avgCycleTimeMs: averageDuration(issueAnalytics.completedDurations),
      completedInPeriod: issueAnalytics.completedInPeriod,
      createdInPeriod: issueAnalytics.createdInPeriod,
      netFlow:
        issueAnalytics.completedInPeriod - issueAnalytics.createdInPeriod,
      overdueIssues: issueAnalytics.overdueIssues,
      statusCounts: issueAnalytics.statusCounts,
    },
    totals: {
      ...scopedTotals,
      projects: projects.length,
      workItems: issueAnalytics.totalIssues,
    },
    trend: issueAnalytics.trend,
  };
};

export const analyticsForWorkspace = query({
  args: {
    projectId: v.optional(v.id("projects")),
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceMembership(ctx, user._id, args.workspaceId);
    const selectedProject = await getSelectedProject(
      ctx,
      args.workspaceId,
      args.projectId
    );

    return await buildWorkspaceAnalytics(ctx, {
      selectedProject,
      workspaceId: args.workspaceId,
    });
  },
});

export const overviewForWorkspace = query({
  args: {
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceMembership(ctx, user._id, args.workspaceId);

    const [issues, states, projects, activities] = await Promise.all([
      ctx.db
        .query("issues")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", args.workspaceId)
        )
        .collect(),
      ctx.db
        .query("issueStates")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", args.workspaceId)
        )
        .collect(),
      ctx.db
        .query("projects")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", args.workspaceId)
        )
        .collect(),
      ctx.db
        .query("issueActivities")
        .withIndex("by_workspaceId", (q) =>
          q.eq("workspaceId", args.workspaceId)
        )
        .order("desc")
        .take(RECENT_ACTIVITY_LIMIT),
    ]);

    const stateById = new Map(states.map((state) => [state._id, state]));
    const projectById = new Map(
      projects.map((project) => [project._id, project])
    );
    const activeIssues = issues.filter((issue) => !issue.archivedAt);
    const now = Date.now();
    const weekAgo = now - WEEK_MS;

    const isClosed = (issue: (typeof activeIssues)[number]) => {
      const type = stateById.get(issue.stateId)?.type;
      return type === "completed" || type === "cancelled";
    };
    const isOverdue = (issue: (typeof activeIssues)[number]) =>
      Boolean(
        !isClosed(issue) &&
          issue.targetDate &&
          new Date(issue.targetDate).getTime() < now
      );

    const completedDurations = issues
      .filter((issue) => issue.completedAt)
      .map(
        (issue) =>
          (issue.completedAt as number) -
          (issue.createdAt ?? issue._creationTime)
      )
      .filter((duration) => duration >= 0);

    const stats = {
      avgCycleTimeMs:
        completedDurations.length > 0
          ? Math.round(
              completedDurations.reduce((sum, value) => sum + value, 0) /
                completedDurations.length
            )
          : null,
      completedThisWeek: activeIssues.filter(
        (issue) => issue.completedAt && issue.completedAt >= weekAgo
      ).length,
      createdThisWeek: issues.filter(
        (issue) => (issue.createdAt ?? issue._creationTime) >= weekAgo
      ).length,
      openIssues: activeIssues.filter((issue) => !isClosed(issue)).length,
      overdueIssues: activeIssues.filter((issue) => isOverdue(issue)).length,
      startedIssues: activeIssues.filter(
        (issue) => stateById.get(issue.stateId)?.type === "started"
      ).length,
    };

    const startOfToday = new Date(now).setHours(0, 0, 0, 0);
    const trend = Array.from({ length: TREND_DAYS }, (_, index) => {
      const dayStart = startOfToday - (TREND_DAYS - 1 - index) * DAY_MS;
      const dayEnd = dayStart + DAY_MS;
      const inDay = (timestamp: number | undefined) =>
        timestamp !== undefined && timestamp >= dayStart && timestamp < dayEnd;

      return {
        completed: issues.filter((issue) => inDay(issue.completedAt)).length,
        created: issues.filter((issue) =>
          inDay(issue.createdAt ?? issue._creationTime)
        ).length,
        date: dayStart,
      };
    });

    const summarizeIssue = (issue: (typeof activeIssues)[number]) => {
      const state = stateById.get(issue.stateId);

      return {
        _id: issue._id,
        assigneeUserId: issue.assigneeUserId ?? null,
        createdAt: issue.createdAt ?? issue._creationTime,
        identifier: issue.identifier,
        overdue: isOverdue(issue),
        priority: issue.priority,
        projectId: issue.projectId,
        projectName: projectById.get(issue.projectId)?.name ?? "Unknown",
        state: state
          ? { color: state.color, name: state.name, type: state.type }
          : null,
        targetDate: issue.targetDate ?? null,
        title: issue.title,
        updatedAt: issue.updatedAt,
      };
    };

    const recentIssues = activeIssues
      .toSorted(
        (left, right) =>
          (right.createdAt ?? right._creationTime) -
          (left.createdAt ?? left._creationTime)
      )
      .slice(0, RECENT_ISSUE_LIMIT)
      .map(summarizeIssue);

    const myIssues = activeIssues
      .filter((issue) => issue.assigneeUserId === user._id && !isClosed(issue))
      .toSorted((left, right) => right.updatedAt - left.updatedAt)
      .slice(0, MY_ISSUE_LIMIT)
      .map(summarizeIssue);

    const projectSummaries = projects.map((project) => {
      const projectIssues = activeIssues.filter(
        (issue) => issue.projectId === project._id
      );
      const completed = projectIssues.filter(
        (issue) => stateById.get(issue.stateId)?.type === "completed"
      ).length;

      return {
        _id: project._id,
        color: project.color ?? null,
        completedIssues: completed,
        name: project.name,
        totalIssues: projectIssues.length,
      };
    });

    const actorIds = [
      ...new Set(activities.map((activity) => activity.actorUserId)),
    ];
    const actorEntries = await Promise.all(
      actorIds.map(
        async (actorId) =>
          [actorId, await authComponent.getAnyUserById(ctx, actorId)] as const
      )
    );
    const actorById = new Map(actorEntries);

    const recentActivity = await Promise.all(
      activities.map(async (activity) => {
        const issue = await ctx.db.get(activity.issueId);
        const actor = actorById.get(activity.actorUserId);

        return {
          _id: activity._id,
          actorImage: actor?.image ?? null,
          actorName: actor?.name ?? actor?.email ?? "Someone",
          createdAt: activity._creationTime,
          issueId: activity.issueId,
          issueIdentifier: issue?.identifier ?? null,
          issueTitle: issue?.title ?? null,
          message: activity.message,
          projectId: issue?.projectId ?? null,
        };
      })
    );

    return {
      myIssues,
      projects: projectSummaries,
      recentActivity,
      recentIssues,
      stats,
      trend,
    };
  },
});
