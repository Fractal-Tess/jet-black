import { ConvexError, v } from "convex/values";

import { mutation } from "../_generated/server";
import { requireAuthUser } from "../lib/auth";
import { requireProjectAccess } from "../lib/workspaceAccess";
import { issueStateType } from "../schema/issues";

const POSITION_STEP = 1000;

export const create = mutation({
  args: {
    color: v.string(),
    name: v.string(),
    projectId: v.id("projects"),
    type: issueStateType,
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const project = await requireProjectAccess(ctx, user._id, args.projectId);
    const name = args.name.trim();

    if (!name) {
      throw new ConvexError("State name is required");
    }

    const states = await ctx.db
      .query("issueStates")
      .withIndex("by_projectId", (q) => q.eq("projectId", project._id))
      .collect();

    if (
      states.some((state) => state.name.toLowerCase() === name.toLowerCase())
    ) {
      throw new ConvexError("A state with this name already exists");
    }

    const maxPosition = Math.max(
      0,
      ...states
        .filter((state) => state.type === args.type)
        .map((state) => state.position)
    );

    return await ctx.db.insert("issueStates", {
      color: args.color,
      isDefault: false,
      name,
      position: maxPosition + POSITION_STEP,
      projectId: project._id,
      type: args.type,
      workspaceId: project.workspaceId,
    });
  },
});

export const update = mutation({
  args: {
    color: v.optional(v.string()),
    name: v.optional(v.string()),
    stateId: v.id("issueStates"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const state = await ctx.db.get(args.stateId);

    if (!state) {
      throw new ConvexError("State not found");
    }

    await requireProjectAccess(ctx, user._id, state.projectId);

    const patch: { color?: string; name?: string } = {};

    if (args.name !== undefined) {
      const name = args.name.trim();

      if (!name) {
        throw new ConvexError("State name is required");
      }

      patch.name = name;
    }

    if (args.color !== undefined) {
      patch.color = args.color;
    }

    await ctx.db.patch(state._id, patch);

    return null;
  },
});

export const setDefault = mutation({
  args: {
    stateId: v.id("issueStates"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const state = await ctx.db.get(args.stateId);

    if (!state) {
      throw new ConvexError("State not found");
    }

    await requireProjectAccess(ctx, user._id, state.projectId);

    const states = await ctx.db
      .query("issueStates")
      .withIndex("by_projectId", (q) => q.eq("projectId", state.projectId))
      .collect();

    await Promise.all(
      states
        .filter((row) => row.isDefault && row._id !== state._id)
        .map((row) => ctx.db.patch(row._id, { isDefault: false }))
    );

    await ctx.db.patch(state._id, { isDefault: true });

    return null;
  },
});

export const reorder = mutation({
  args: {
    position: v.number(),
    stateId: v.id("issueStates"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const state = await ctx.db.get(args.stateId);

    if (!state) {
      throw new ConvexError("State not found");
    }

    await requireProjectAccess(ctx, user._id, state.projectId);
    await ctx.db.patch(state._id, { position: args.position });

    return null;
  },
});

export const remove = mutation({
  args: {
    stateId: v.id("issueStates"),
  },
  handler: async (ctx, args) => {
    const user = await requireAuthUser(ctx);
    const state = await ctx.db.get(args.stateId);

    if (!state) {
      throw new ConvexError("State not found");
    }

    await requireProjectAccess(ctx, user._id, state.projectId);

    if (state.isDefault) {
      throw new ConvexError("The default state cannot be deleted");
    }

    const referencingIssue = await ctx.db
      .query("issues")
      .withIndex("by_projectId_stateId_position", (q) =>
        q.eq("projectId", state.projectId).eq("stateId", state._id)
      )
      .first();

    if (referencingIssue) {
      throw new ConvexError(
        "This state is used by existing issues and cannot be deleted"
      );
    }

    const remaining = await ctx.db
      .query("issueStates")
      .withIndex("by_projectId", (q) => q.eq("projectId", state.projectId))
      .take(2);

    if (remaining.length <= 1) {
      throw new ConvexError("A project needs at least one state");
    }

    await ctx.db.delete(state._id);

    return null;
  },
});
