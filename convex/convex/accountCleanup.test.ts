/// <reference types="vite/client" />

import { convexTest } from "convex-test";
import { describe, expect, it } from "vitest";
import { internal } from "./_generated/api.js";
import type { Id } from "./_generated/dataModel.js";
import schema from "./schema.js";

const modules = import.meta.glob("./**/*.*s");

function setup() {
  return convexTest({ schema, modules });
}

async function seedWorkspaceWithOwner(
  t: ReturnType<typeof setup>,
  userId: string,
  workspaceName: string
) {
  return await t.run(async (ctx) => {
    const workspaceId = await ctx.db.insert("workspaces", {
      createdByUserId: userId,
      name: workspaceName,
      slug: workspaceName.toLowerCase().replace(/\s+/g, "-"),
      updatedAt: Date.now(),
    });
    await ctx.db.insert("workspaceMembers", {
      role: "owner",
      userId,
      workspaceId,
    });
    const projectId = await ctx.db.insert("projects", {
      color: "#38bdf8",
      key: "TST",
      name: "Test Project",
      slug: "test-project",
      updatedAt: Date.now(),
      workspaceId,
    });
    const stateId = await ctx.db.insert("issueStates", {
      color: "#999",
      isDefault: true,
      name: "Todo",
      position: 0,
      projectId,
      type: "unstarted",
      workspaceId,
    });
    await ctx.db.insert("issues", {
      createdByUserId: userId,
      identifier: "TST-1",
      priority: "none",
      projectId,
      sequenceId: 1,
      stateId,
      title: "Test issue",
      updatedAt: Date.now(),
      workspaceId,
    });
    return { projectId, workspaceId };
  });
}

async function addMember(
  t: ReturnType<typeof setup>,
  userId: string,
  workspaceId: Id<"workspaces">
) {
  await t.run(async (ctx) => {
    await ctx.db.insert("workspaceMembers", {
      role: "member",
      userId,
      workspaceId,
    });
  });
}

describe("deleteForUser", () => {
  it("sole owner with no other members: full cleanup", async () => {
    const t = setup();
    const userId = "user_sole_owner";
    const { workspaceId } = await seedWorkspaceWithOwner(
      t,
      userId,
      "Solo Workspace"
    );

    await t.mutation(internal.mutations.accountCleanup.deleteForUser, {
      userId,
    });

    await t.run(async (ctx) => {
      const workspace = await ctx.db.get(workspaceId);
      expect(workspace).toBeNull();

      const members = await ctx.db
        .query("workspaceMembers")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(members).toHaveLength(0);

      const projects = await ctx.db
        .query("projects")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(projects).toHaveLength(0);

      const issues = await ctx.db
        .query("issues")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(issues).toHaveLength(0);

      const states = await ctx.db
        .query("issueStates")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(states).toHaveLength(0);
    });
  });

  it("owner with other members: blocks deletion", async () => {
    const t = setup();
    const ownerId = "user_owner";
    const memberId = "user_member";
    const { workspaceId } = await seedWorkspaceWithOwner(
      t,
      ownerId,
      "Shared Workspace"
    );
    await addMember(t, memberId, workspaceId);

    await expect(
      t.mutation(internal.mutations.accountCleanup.deleteForUser, {
        userId: ownerId,
      })
    ).rejects.toThrow(
      "Cannot delete account while you own a workspace with other members"
    );

    await t.run(async (ctx) => {
      const workspace = await ctx.db.get(workspaceId);
      expect(workspace).not.toBeNull();

      const members = await ctx.db
        .query("workspaceMembers")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(members).toHaveLength(2);
    });
  });

  it("non-owner member: removes only their membership", async () => {
    const t = setup();
    const ownerId = "user_owner";
    const memberId = "user_member";
    const { workspaceId } = await seedWorkspaceWithOwner(
      t,
      ownerId,
      "Team Workspace"
    );
    await addMember(t, memberId, workspaceId);

    await t.mutation(internal.mutations.accountCleanup.deleteForUser, {
      userId: memberId,
    });

    await t.run(async (ctx) => {
      const workspace = await ctx.db.get(workspaceId);
      expect(workspace).not.toBeNull();

      const members = await ctx.db
        .query("workspaceMembers")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(members).toHaveLength(1);
      const remainingMember = members[0];
      if (remainingMember === undefined) {
        throw new Error(
          `Expected one remaining member in workspace "${workspaceId}" but found none`
        );
      }
      expect(remainingMember.userId).toBe(ownerId);
      expect(remainingMember.role).toBe("owner");

      const projects = await ctx.db
        .query("projects")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(projects).toHaveLength(1);

      const issues = await ctx.db
        .query("issues")
        .withIndex("by_workspaceId", (q) => q.eq("workspaceId", workspaceId))
        .collect();
      expect(issues).toHaveLength(1);
    });
  });
});
