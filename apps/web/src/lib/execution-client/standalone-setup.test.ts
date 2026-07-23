/// <reference types="bun" />

import { describe, expect, test } from "bun:test";
import {
  type Envelope,
  type LocalCommand,
  PROTOCOL_VERSION,
} from "@workspace/shared/protocol";
import { createHttpExecutionClient } from "./http";
import {
  listApprovedRepositories,
  startStandaloneRun,
} from "./standalone-setup";

const REQUEST_IDS = [
  "123e4567-e89b-42d3-a456-426614174010",
  "123e4567-e89b-42d3-a456-426614174011",
  "123e4567-e89b-42d3-a456-426614174012",
];

const APPROVED_REPOSITORY = {
  id: "approved-repository-id",
  display_name: "Approved repository",
};

const responseData = (command: LocalCommand): unknown => {
  switch (command.type) {
    case "list_approved_repositories":
      return {
        type: "approved_repositories",
        data: { repositories: [APPROVED_REPOSITORY] },
      };
    case "register_repository":
      return {
        type: "repository_registered",
        data: {
          id: "repository-id",
          default_branch: "main",
          base_sha: "base-sha",
          version: 0,
        },
      };
    case "create_changeset":
      return {
        type: "changeset_created",
        data: {
          id: "changeset-id",
          repository_id: command.data.repository_id,
          base_sha: command.data.base_sha,
          head_sha: command.data.base_sha,
          state: "created",
          ticket: null,
          version: 0,
        },
      };
    case "start_run":
      return {
        type: "run_started",
        data: {
          run_id: "run-id",
          changeset_id: command.data.changeset_id,
          worktree_id: "worktree-id",
          approval_id: null,
          approval_request: null,
        },
      };
    default:
      throw new Error(`Unexpected command: ${command.type}`);
  }
};

describe("standalone run setup", () => {
  test("uses returned identities for each setup command", async () => {
    const commands: LocalCommand[] = [];
    let requestIndex = 0;
    const client = createHttpExecutionClient({
      endpoint: "/api/commands",
      createRequestId: () => REQUEST_IDS[requestIndex++] ?? crypto.randomUUID(),
      fetch: (_input, init) => {
        const envelope = JSON.parse(
          String(init?.body)
        ) as Envelope<LocalCommand>;
        commands.push(envelope.payload);
        return Promise.resolve(
          Response.json({
            version: PROTOCOL_VERSION,
            request_id: envelope.request_id,
            result: { status: "ok", data: responseData(envelope.payload) },
          })
        );
      },
    });

    const setup = await startStandaloneRun(client, APPROVED_REPOSITORY);

    expect(commands).toEqual([
      {
        type: "register_repository",
        data: { approved_repository_id: APPROVED_REPOSITORY.id },
      },
      {
        type: "create_changeset",
        data: {
          repository_id: "repository-id",
          base_sha: "base-sha",
          ticket: null,
        },
      },
      {
        type: "start_run",
        data: { changeset_id: "changeset-id" },
      },
    ]);
    expect(setup.run.run_id).toBe("run-id");
    expect(setup.changeset.repository_id).toBe(setup.repository.id);
  });

  test("loads approved repositories through the typed command", async () => {
    const client = createHttpExecutionClient({
      endpoint: "/api/commands",
      createRequestId: () => REQUEST_IDS[0] ?? crypto.randomUUID(),
      fetch: (_input, init) => {
        const envelope = JSON.parse(
          String(init?.body)
        ) as Envelope<LocalCommand>;
        return Promise.resolve(
          Response.json({
            version: PROTOCOL_VERSION,
            request_id: envelope.request_id,
            result: { status: "ok", data: responseData(envelope.payload) },
          })
        );
      },
    });

    await expect(listApprovedRepositories(client)).resolves.toEqual([
      APPROVED_REPOSITORY,
    ]);
  });

  test("rejects an empty repository selection before dispatch", async () => {
    const client = createHttpExecutionClient({
      endpoint: "/api/commands",
      fetch: () => {
        throw new Error("command should not be sent");
      },
    });

    try {
      await startStandaloneRun(client, {
        id: "   ",
        display_name: "Invalid repository",
      });
      throw new Error("Expected an empty repository selection to be rejected.");
    } catch (error) {
      expect(error).toEqual(
        expect.objectContaining({ code: "invalid_repository_selection" })
      );
    }
  });
});
