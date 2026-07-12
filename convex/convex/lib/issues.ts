import { ConvexError } from "convex/values";

import type { Id } from "../_generated/dataModel";
import type { MutationCtx } from "../_generated/server";

export async function getDefaultStateId(
  ctx: MutationCtx,
  projectId: Id<"projects">
) {
  const states = await ctx.db
    .query("issueStates")
    .withIndex("by_projectId", (q) => q.eq("projectId", projectId))
    .collect();
  const defaultState = states.find((state) => state.isDefault) ?? states[0];

  if (!defaultState) {
    throw new ConvexError("Project has no issue states");
  }

  return defaultState._id;
}

export async function getNextSequenceId(
  ctx: MutationCtx,
  projectId: Id<"projects">
) {
  const latestIssue = await ctx.db
    .query("issues")
    .withIndex("by_projectId_sequenceId", (q) => q.eq("projectId", projectId))
    .order("desc")
    .first();

  return (latestIssue?.sequenceId ?? 0) + 1;
}
