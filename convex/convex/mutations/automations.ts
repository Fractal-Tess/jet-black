import { v } from "convex/values";

import { internal } from "../_generated/api";
import type { Doc } from "../_generated/dataModel";
import { internalMutation, type MutationCtx } from "../_generated/server";

const MONTH_MS = 30 * 24 * 60 * 60 * 1000;
const ISSUE_BATCH = 200;

export const runAll = internalMutation({
  args: {},
  handler: async (ctx) => {
    for await (const project of ctx.db.query("projects")) {
      const automations = project.automations;
      const enabled =
        automations &&
        (automations.autoArchiveClosedMonths !== null ||
          automations.autoCloseInactiveMonths !== null);

      if (enabled && !project.archivedAt) {
        await ctx.scheduler.runAfter(
          0,
          internal.mutations.automations.runForProject,
          { projectId: project._id }
        );
      }
    }
  },
});

async function autoArchiveIssue(
  ctx: MutationCtx,
  issue: Doc<"issues">,
  now: number
) {
  await ctx.db.patch(issue._id, { archivedAt: now, updatedAt: now });
  await ctx.db.insert("issueActivities", {
    actorUserId: "automation",
    issueId: issue._id,
    message: "automatically archived this closed issue",
    workspaceId: issue.workspaceId,
  });
}

async function autoCloseIssue(
  ctx: MutationCtx,
  issue: Doc<"issues">,
  cancelledStateId: Doc<"issueStates">["_id"],
  now: number
) {
  await ctx.db.patch(issue._id, {
    stateId: cancelledStateId,
    updatedAt: now,
  });
  await ctx.db.insert("issueActivities", {
    actorUserId: "automation",
    issueId: issue._id,
    message: "automatically closed this issue due to inactivity",
    workspaceId: issue.workspaceId,
  });
}

export const runForProject = internalMutation({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const project = await ctx.db.get(args.projectId);

    if (!project?.automations) {
      return;
    }

    const { autoArchiveClosedMonths, autoCloseInactiveMonths } =
      project.automations;
    const now = Date.now();
    const states = await ctx.db
      .query("issueStates")
      .withIndex("by_projectId", (q) => q.eq("projectId", project._id))
      .collect();
    const typeByStateId = new Map(
      states.map((state) => [state._id, state.type])
    );
    const cancelledState = states
      .filter((state) => state.type === "cancelled")
      .sort((a, b) => a.position - b.position)[0];

    let processed = 0;

    for await (const issue of ctx.db
      .query("issues")
      .withIndex("by_projectId", (q) => q.eq("projectId", project._id))) {
      if (processed >= ISSUE_BATCH) {
        break;
      }

      if (issue.archivedAt) {
        continue;
      }

      const stateType = typeByStateId.get(issue.stateId);
      const lastTouched = issue.completedAt ?? issue.updatedAt;

      if (
        autoArchiveClosedMonths !== null &&
        stateType === "completed" &&
        lastTouched < now - autoArchiveClosedMonths * MONTH_MS
      ) {
        await autoArchiveIssue(ctx, issue, now);
        processed += 1;
        continue;
      }

      if (
        autoCloseInactiveMonths !== null &&
        cancelledState &&
        stateType !== "completed" &&
        stateType !== "cancelled" &&
        issue.updatedAt < now - autoCloseInactiveMonths * MONTH_MS
      ) {
        await autoCloseIssue(ctx, issue, cancelledState._id, now);
        processed += 1;
      }
    }

    if (processed >= ISSUE_BATCH) {
      await ctx.scheduler.runAfter(
        0,
        internal.mutations.automations.runForProject,
        { projectId: project._id }
      );
    }
  },
});
