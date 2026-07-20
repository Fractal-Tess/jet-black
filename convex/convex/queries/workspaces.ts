import { v } from "convex/values";

import { query } from "../_generated/server";
import { authComponent } from "../auth";
import { requireAuthUser } from "../lib/auth";
import {
  requireProjectAccess,
  requireWorkspaceMembership,
  requireWorkspaceRole,
} from "../lib/workspaceAccess";

export const viewer = query({
  args: { slug: v.optional(v.string()) },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const memberships = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_userId", (q) => q.eq("userId", user._id))
      .collect();

    const resolved = (
      await Promise.all(
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
      )
    ).filter((item): item is NonNullable<typeof item> => item !== null);

    const active = args.slug
      ? (resolved.find((item) => item.workspace.slug === args.slug) ?? null)
      : (resolved[0] ?? null);

    const projects = active
      ? (
          await ctx.db
            .query("projects")
            .withIndex("by_workspaceId", (q) =>
              q.eq("workspaceId", active.workspace._id)
            )
            .collect()
        ).filter((project) => !project.archivedAt)
      : [];

    const allWorkspaces = resolved.map((item) => item.workspace);

    const allMemberships = resolved.map((item) => ({
      role: item.membership.role,
      workspaceId: item.membership.workspaceId,
    }));

    return {
      activeProject: projects[0] ?? null,
      activeWorkspace: active?.workspace ?? null,
      memberships: allMemberships,
      projects,
      user: {
        email: user.email,
        id: user._id,
        image: user.image ?? null,
        name: user.name,
      },
      workspaces: allWorkspaces,
    };
  },
});

export const membersForWorkspace = query({
  args: {
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceMembership(ctx, user._id, args.workspaceId);

    const memberships = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_workspaceId", (q) => q.eq("workspaceId", args.workspaceId))
      .collect();

    return await Promise.all(
      memberships.map(async (membership) => {
        const member = await authComponent.getAnyUserById(
          ctx,
          membership.userId
        );

        return {
          id: membership.userId,
          role: membership.role,
          image: member?.image ?? null,
          email: member?.email ?? "",
          name: member?.name ?? member?.email ?? "Former member",
        };
      })
    );
  },
});

export const invitesForWorkspace = query({
  args: {
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    await requireWorkspaceRole(ctx, user._id, args.workspaceId, "admin");

    const invites = await ctx.db
      .query("workspaceInvites")
      .withIndex("by_workspaceId", (q) => q.eq("workspaceId", args.workspaceId))
      .collect();

    return await Promise.all(
      invites.map(async (invite) => {
        const inviter = await authComponent.getAnyUserById(
          ctx,
          invite.invitedByUserId
        );

        return {
          _creationTime: invite._creationTime,
          _id: invite._id,
          email: invite.email,
          invitedBy: inviter?.name ?? inviter?.email ?? "Former member",
          role: invite.role,
        };
      })
    );
  },
});

export const workspaceBySlug = query({
  args: { slug: v.string() },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);

    const workspace = await ctx.db
      .query("workspaces")
      .withIndex("by_slug", (q) => q.eq("slug", args.slug))
      .unique();

    if (!workspace) {
      return null;
    }

    const membership = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_workspaceId_userId", (q) =>
        q.eq("workspaceId", workspace._id).eq("userId", user._id)
      )
      .unique();

    if (!membership) {
      return null;
    }

    const projects = (
      await ctx.db
        .query("projects")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspace._id))
        .collect()
    ).filter((project) => !project.archivedAt);

    return {
      membership,
      projects,
      user: {
        email: user.email,
        id: user._id,
        image: user.image ?? null,
        name: user.name,
      },
      workspace,
    };
  },
});

export const projectWithAccess = query({
  args: {
    projectId: v.id("projects"),
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);

    if (project.workspaceId !== args.workspaceId) {
      return null;
    }

    return project;
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
