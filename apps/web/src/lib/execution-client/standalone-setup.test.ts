/// <reference types="bun" />

import { describe, expect, test } from "bun:test";
import {
  type Envelope,
  type LocalCommand,
  PROTOCOL_VERSION,
} from "@workspace/shared/protocol";
import { createHttpExecutionClient } from "./http";
import { startStandaloneRun } from "./standalone-setup";

const REQUEST_IDS = [
  "123e4567-e89b-42d3-a456-426614174010",
  "123e4567-e89b-42d3-a456-426614174011",
  "123e4567-e89b-42d3-a456-426614174012",
];

const responseData = (command: LocalCommand): unknown => {
  switch (command.type) {
    case "register_repository":
      return {
        type: "repository_registered",
        data: {
          id: "repository-id",
          canonical_path: "/approved/repository",
          filesystem_identity: "filesystem",
          git_directory_identity: "git-directory",
          identity: "repository",
          primary_remote: null,
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

    const setup = await startStandaloneRun(client, "/approved/repository");

    expect(commands).toEqual([
      {
        type: "register_repository",
        data: { path: "/approved/repository" },
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

  test("rejects blank paths before dispatch", async () => {
    const client = createHttpExecutionClient({
      endpoint: "/api/commands",
      fetch: () => {
        throw new Error("command should not be sent");
      },
    });

    try {
      await startStandaloneRun(client, "   ");
      throw new Error("Expected blank repository path to be rejected.");
    } catch (error) {
      expect(error).toEqual(
        expect.objectContaining({ code: "invalid_repository_path" })
      );
    }
  });
});
