import type { Id } from "../_generated/dataModel";
import type { MutationCtx } from "../_generated/server";
import { DEFAULT_STATES } from "./defaultWorkspace";

export async function createDefaultIssueStates(
  ctx: MutationCtx,
  projectId: Id<"projects">,
  workspaceId: Id<"workspaces">
) {
  for (const state of DEFAULT_STATES) {
    await ctx.db.insert("issueStates", {
      ...state,
      projectId,
      workspaceId,
    });
  }
}
