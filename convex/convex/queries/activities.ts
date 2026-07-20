import { v } from "convex/values";

import { query } from "../_generated/server";
import { authComponent } from "../auth";
import { requireAuthUser } from "../lib/auth";
import { requireIssueAccess } from "../lib/workspaceAccess";

export const listForIssue = query({
  args: {
    issueId: v.id("issues"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireIssueAccess(ctx, user._id, args.issueId);

    const activities = await ctx.db
      .query("issueActivities")
      .withIndex("by_issueId", (q) => q.eq("issueId", args.issueId))
      .order("asc")
      .collect();

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

    return activities.map((activity) => {
      const actor = actorById.get(activity.actorUserId);

      return {
        ...activity,
        actor: {
          image: actor?.image ?? null,
          name: actor?.name ?? "Former member",
        },
      };
    });
  },
});
