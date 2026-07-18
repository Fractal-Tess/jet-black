import { v } from "convex/values";

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
