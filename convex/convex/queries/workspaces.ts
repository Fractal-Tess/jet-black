import { v } from "convex/values";

import { query } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireProjectAccess } from "../lib/workspaceAccess";

export const viewer = query({
  args: {},
  handler: async (ctx) => {
    const user = await requireAuthUser(ctx);
    const memberships = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_userId", (q) => q.eq("userId", user._id))
      .collect();

    const workspaces = await Promise.all(
      memberships.map(async (membership) => {
        const workspace = await ctx.db.get(membership.workspaceId);
        if (!workspace) {
          return null;
        }

        return {
          membership,
          workspace,
        };
      })
    );

    const active = workspaces.find((item) => item?.workspace) ?? null;
    const projects = active
      ? await ctx.db
          .query("projects")
          .withIndex("by_workspaceId", (q) =>
            q.eq("workspaceId", active.workspace._id)
          )
          .collect()
      : [];

    return {
      activeProject: projects[0] ?? null,
      activeWorkspace: active?.workspace ?? null,
      projects,
      user: {
        email: user.email,
        id: user._id,
        image: user.image ?? null,
        name: user.name,
      },
    };
  },
});

export const statesForProject = query({
  args: {
    projectId: v.id("projects"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireProjectAccess(ctx, user._id, args.projectId);

    return await ctx.db
      .query("issueStates")
      .withIndex("by_projectId", (q) => q.eq("projectId", args.projectId))
      .collect();
  },
});
