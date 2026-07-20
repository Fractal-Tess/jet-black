import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { authComponent } from "../auth";
import { requireAuthUser } from "../lib/auth";
import { ROLE_RANK, requireWorkspaceRole } from "../lib/workspaceAccess";
import { workspaceRole } from "../schema/workspaces";

export const invite = mutation({
  args: {
    email: v.string(),
    role: workspaceRole,
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const actor = await requireWorkspaceRole(
      ctx,
      user._id,
      args.workspaceId,
      "admin"
    );

    if (args.role === "owner") {
      throw new ConvexError("Cannot invite a member as owner");
    }

    if (ROLE_RANK[actor.role] < ROLE_RANK[args.role]) {
      throw new ConvexError(
        "Cannot invite a member with a higher role than your own"
      );
    }

    const email = args.email.trim().toLowerCase();

    if (!email.includes("@")) {
      throw new ConvexError("A valid email address is required");
    }

    const memberships = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_workspaceId", (q) => q.eq("workspaceId", args.workspaceId))
      .collect();

    for (const membership of memberships) {
      const member = await authComponent.getAnyUserById(ctx, membership.userId);
      if (member?.email?.toLowerCase() === email) {
        throw new ConvexError("This person is already a workspace member");
      }
    }

    const existingInvite = await ctx.db
      .query("workspaceInvites")
      .withIndex("by_workspaceId_email", (q) =>
        q.eq("workspaceId", args.workspaceId).eq("email", email)
      )
      .unique();

    if (existingInvite) {
      await ctx.db.patch(existingInvite._id, {
        invitedByUserId: user._id,
        role: args.role,
      });
      return existingInvite._id;
    }

    return await ctx.db.insert("workspaceInvites", {
      email,
      invitedByUserId: user._id,
      role: args.role,
      workspaceId: args.workspaceId,
    });
  },
});

export const revokeInvite = mutation({
  args: {
    inviteId: v.id("workspaceInvites"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const inviteRecord = await ctx.db.get(args.inviteId);

    if (!inviteRecord) {
      throw new ConvexError("Invite not found");
    }

    await requireWorkspaceRole(
      ctx,
      user._id,
      inviteRecord.workspaceId,
      "admin"
    );
    await ctx.db.delete(args.inviteId);

    return null;
  },
});

export const updateRole = mutation({
  args: {
    memberUserId: v.string(),
    role: workspaceRole,
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const actor = await requireWorkspaceRole(
      ctx,
      user._id,
      args.workspaceId,
      "admin"
    );

    if (args.memberUserId === user._id) {
      throw new ConvexError("You cannot change your own role");
    }

    if (args.role === "owner") {
      throw new ConvexError("Ownership cannot be transferred here");
    }

    const target = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_workspaceId_userId", (q) =>
        q.eq("workspaceId", args.workspaceId).eq("userId", args.memberUserId)
      )
      .unique();

    if (!target) {
      throw new ConvexError("Member not found");
    }

    const actorRank = ROLE_RANK[actor.role];

    if (actorRank <= ROLE_RANK[target.role]) {
      throw new ConvexError(
        "You can only change roles of members below your role"
      );
    }

    if (actorRank < ROLE_RANK[args.role]) {
      throw new ConvexError("Cannot grant a role higher than your own");
    }

    await ctx.db.patch(target._id, { role: args.role });

    return null;
  },
});

export const removeMember = mutation({
  args: {
    memberUserId: v.string(),
    workspaceId: v.id("workspaces"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const actor = await requireWorkspaceRole(
      ctx,
      user._id,
      args.workspaceId,
      "admin"
    );

    if (args.memberUserId === user._id) {
      throw new ConvexError("You cannot remove yourself");
    }

    const target = await ctx.db
      .query("workspaceMembers")
      .withIndex("by_workspaceId_userId", (q) =>
        q.eq("workspaceId", args.workspaceId).eq("userId", args.memberUserId)
      )
      .unique();

    if (!target) {
      throw new ConvexError("Member not found");
    }

    if (target.role === "owner") {
      throw new ConvexError("The workspace owner cannot be removed");
    }

    if (ROLE_RANK[actor.role] <= ROLE_RANK[target.role]) {
      throw new ConvexError("You can only remove members below your role");
    }

    const projectRoster = await ctx.db
      .query("projectMembers")
      .withIndex("by_workspaceId_userId", (q) =>
        q.eq("workspaceId", args.workspaceId).eq("userId", args.memberUserId)
      )
      .collect();

    await Promise.all([
      ctx.db.delete(target._id),
      ...projectRoster.map((row) => ctx.db.delete(row._id)),
    ]);

    return null;
  },
});

export const claimInvites = mutation({
  args: {},
  handler: async (ctx) => {
    const user = await requireAuthUser(ctx);
    const email = user.email?.trim().toLowerCase();

    if (!email) {
      return { claimed: 0 };
    }

    const invites = await ctx.db
      .query("workspaceInvites")
      .withIndex("by_email", (q) => q.eq("email", email))
      .collect();

    let claimed = 0;

    for (const inviteRecord of invites) {
      const workspace = await ctx.db.get(inviteRecord.workspaceId);

      if (workspace) {
        const existing = await ctx.db
          .query("workspaceMembers")
          .withIndex("by_workspaceId_userId", (q) =>
            q.eq("workspaceId", inviteRecord.workspaceId).eq("userId", user._id)
          )
          .unique();

        if (!existing) {
          await ctx.db.insert("workspaceMembers", {
            role: inviteRecord.role,
            userId: user._id,
            workspaceId: inviteRecord.workspaceId,
          });
          claimed += 1;
        }
      }

      await ctx.db.delete(inviteRecord._id);
    }

    return { claimed };
  },
});
