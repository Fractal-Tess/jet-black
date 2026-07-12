import type { MutationCtx } from "./_generated/server";
import { internalMutation } from "./_generated/server";
import { DEFAULT_PROJECT_KEY } from "./lib/defaultWorkspace";
import { buildSeedMessage } from "./lib/messages";
import { createSampleIssues } from "./lib/sampleIssues";
import { createDefaultIssueStates } from "./lib/states";

const DEV_SEED_USER_ID = "dev-seed-user";
const DEV_SEED_WORKSPACE_SLUG = "jet-black-dev-seed";

async function seedDevData(ctx: MutationCtx) {
  const existingWorkspace = await ctx.db
    .query("workspaces")
    .withIndex("by_slug", (q) => q.eq("slug", DEV_SEED_WORKSPACE_SLUG))
    .unique();

  if (existingWorkspace) {
    return {
      seeded: false,
      workspaceId: existingWorkspace._id,
    };
  }

  const now = Date.now();
  const workspaceId = await ctx.db.insert("workspaces", {
    createdByUserId: DEV_SEED_USER_ID,
    name: "Jet Black Demo",
    slug: DEV_SEED_WORKSPACE_SLUG,
    updatedAt: now,
  });

  await ctx.db.insert("workspaceMembers", {
    role: "owner",
    userId: DEV_SEED_USER_ID,
    workspaceId,
  });

  const projectId = await ctx.db.insert("projects", {
    color: "#f59e0b",
    description: "Demo issue tracker data for local development.",
    key: DEFAULT_PROJECT_KEY,
    name: "Jet Black",
    slug: "jet-black-demo",
    updatedAt: now,
    workspaceId,
  });

  await createDefaultIssueStates(ctx, projectId, workspaceId);
  await createSampleIssues(ctx, {
    actorUserId: DEV_SEED_USER_ID,
    projectId,
    projectKey: DEFAULT_PROJECT_KEY,
    workspaceId,
  });

  return {
    projectId,
    seeded: true,
    workspaceId,
  };
}

const seedMutation = internalMutation({
  args: {},
  handler: async (ctx) => {
    let messagesInserted = 0;

    const existingMessage = await ctx.db.query("messages").take(1);
    if (existingMessage.length === 0) {
      await ctx.db.insert("messages", {
        body: buildSeedMessage("web app"),
        source: "seed",
      });
      messagesInserted = 1;
    }

    return {
      devData: await seedDevData(ctx),
      messagesInserted,
      seeded: messagesInserted > 0,
    };
  },
});

export const seed = seedMutation;

export default seedMutation;
