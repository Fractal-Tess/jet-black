import type { Id } from "../_generated/dataModel";
import type { MutationCtx } from "../_generated/server";

export const SAMPLE_ISSUES = [
  {
    description:
      "The sign-in flow should create a session and redirect to the dashboard.",
    priority: "high" as const,
    stateName: "In progress",
    title: "Finish email/password login",
  },
  {
    description:
      "New issues, comments, and state changes should update without a refresh.",
    priority: "urgent" as const,
    stateName: "Todo",
    title: "Wire realtime issue updates",
  },
  {
    description:
      "A future iteration should associate issues with GitHub pull requests.",
    priority: "medium" as const,
    stateName: "Done",
    title: "Sketch pull request review workflow",
  },
] as const;

export async function createSampleIssues(
  ctx: MutationCtx,
  input: {
    actorUserId: string;
    projectId: Id<"projects">;
    projectKey: string;
    workspaceId: Id<"workspaces">;
  }
) {
  const states = await ctx.db
    .query("issueStates")
    .withIndex("by_projectId", (q) => q.eq("projectId", input.projectId))
    .collect();
  const fallbackState =
    states.find((state) => state.name === "Todo") ?? states[0];

  if (!fallbackState) {
    throw new Error("Default issue states were not created.");
  }

  const now = Date.now();

  for (const [index, issueInput] of SAMPLE_ISSUES.entries()) {
    const state =
      states.find((candidate) => candidate.name === issueInput.stateName) ??
      fallbackState;
    const sequenceId = index + 1;
    const issueId = await ctx.db.insert("issues", {
      createdByUserId: input.actorUserId,
      description: issueInput.description,
      identifier: `${input.projectKey}-${sequenceId}`,
      position: now + sequenceId,
      priority: issueInput.priority,
      projectId: input.projectId,
      sequenceId,
      stateId: state._id,
      title: issueInput.title,
      updatedAt: now + sequenceId,
      workspaceId: input.workspaceId,
    });

    await ctx.db.insert("issueActivities", {
      actorUserId: input.actorUserId,
      issueId,
      message: "seeded the issue",
      workspaceId: input.workspaceId,
    });

    await ctx.db.insert("issueComments", {
      authorUserId: input.actorUserId,
      body: "Seeded locally so the dashboard has useful data to inspect.",
      issueId,
      updatedAt: now + sequenceId,
      workspaceId: input.workspaceId,
    });
  }
}
