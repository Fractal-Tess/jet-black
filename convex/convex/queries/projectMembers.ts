import { v } from "convex/values";

import { query } from "../_generated/server";
import { authComponent } from "../auth";
import { requireAuthUser } from "../lib/auth";
import { requireProjectAccess } from "../lib/workspaceAccess";

export const listForProject = query({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireProjectAccess(ctx, user._id, args.projectId);

    const memberships = await ctx.db
      .query("projectMembers")
      .withIndex("by_projectId", (q) => q.eq("projectId", args.projectId))
      .collect();

    return await Promise.all(
      memberships.map(async (membership) => {
        const member = await authComponent.getAnyUserById(
          ctx,
          membership.userId
        );

        return {
          _id: membership._id,
          email: member?.email ?? "",
          id: membership.userId,
          image: member?.image ?? null,
          name: member?.name ?? member?.email ?? "Former member",
          role: membership.role,
        };
      })
    );
  },
});
