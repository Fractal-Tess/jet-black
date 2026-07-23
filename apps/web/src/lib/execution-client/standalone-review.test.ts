/// <reference types="bun" />

import { describe, expect, test } from "bun:test";
import {
  type Changeset,
  type Envelope,
  type LocalCommand,
  type MutationPreview,
  type MutationResult,
  PROTOCOL_VERSION,
  type ReviewCheckKind,
  type ReviewReport,
} from "@workspace/shared/protocol";
import { createHttpExecutionClient } from "./http";
import {
  completeChangesetMutation,
  loadStandaloneReview,
  previewChangesetMutation,
  runStandaloneReview,
} from "./standalone-review";

const CHANGESET_ID = "changeset-id";

const changeset = (): Changeset => ({
  id: CHANGESET_ID,
  repository_id: "repository-id",
  base_sha: "base-sha",
  head_sha: "head-sha",
  state: "reviewable",
  ticket: null,
  version: 4,
});

const reviewChecks: ReviewCheckKind[] = [
  "format",
  "typecheck",
  "test",
  "secret_scan",
  "dependency_audit",
];

const reviewReport = (): ReviewReport => ({
  changeset_id: CHANGESET_ID,
  changeset_version: 4,
  head_sha: "head-sha",
  changed_paths: ["README.md"],
  unified_diff: "diff",
  checks: reviewChecks.map((kind) => ({
    kind,
    status: "passed",
    evidence: "passed",
  })),
  findings: [],
});

const mutationPreview = (
  kind: MutationPreview["kind"] = "commit"
): MutationPreview => ({
  kind,
  changeset_id: CHANGESET_ID,
  checkpoint_id: "checkpoint-id",
  expected_version: 4,
  expected_head_sha: "head-sha",
  manifest_sha256: "manifest-sha",
  confirmation_digest: `${kind}-confirmation-digest`,
});

const mutationResult = (
  kind: MutationResult["kind"] = "commit"
): MutationResult => {
  const common = {
    confirmation_digest: `${kind}-confirmation-digest`,
    checkpoint_id: "checkpoint-id",
    manifest_sha256: "manifest-sha",
    changeset: {
      ...changeset(),
      head_sha: kind === "commit" ? "committed-head-sha" : "head-sha",
      state:
        kind === "commit" ? ("committed" as const) : ("discarded" as const),
      version: 5,
    },
    worktree_state: "removed" as const,
  };

  return kind === "commit"
    ? {
        kind,
        ...common,
        resulting_head_sha: "committed-head-sha",
        app_ref: "refs/heads/jet-black/changeset-id",
      }
    : { kind, ...common };
};

const responseData = (command: LocalCommand): unknown => {
  switch (command.type) {
    case "get_diff":
      return {
        type: "diff",
        data: { changeset_id: command.data.changeset_id, unified_diff: "diff" },
      };
    case "get_findings":
      return {
        type: "findings",
        data: { changeset_id: command.data.changeset_id, findings: [] },
      };
    case "get_history":
      return {
        type: "history",
        data: { changeset_id: command.data.changeset_id, runs: [] },
      };
    case "get_recovery":
      return { type: "recovery", data: { actions: [] } };
    case "review_changeset":
      return { type: "review_completed", data: reviewReport() };
    case "preview_commit":
      return { type: "mutation_preview", data: mutationPreview("commit") };
    case "preview_discard":
      return { type: "mutation_preview", data: mutationPreview("discard") };
    case "commit_changeset":
      return { type: "mutation_completed", data: mutationResult("commit") };
    case "discard_changeset":
      return { type: "mutation_completed", data: mutationResult("discard") };
    default:
      throw new Error(`Unexpected command: ${command.type}`);
  }
};

const createClient = (
  commands: LocalCommand[],
  respond: (command: LocalCommand) => unknown = responseData
) =>
  createHttpExecutionClient({
    endpoint: "/api/commands",
    fetch: (_input, init) => {
      const envelope = JSON.parse(String(init?.body)) as Envelope<LocalCommand>;
      commands.push(envelope.payload);
      return Promise.resolve(
        Response.json({
          version: PROTOCOL_VERSION,
          request_id: envelope.request_id,
          result: { status: "ok", data: respond(envelope.payload) },
        })
      );
    },
  });

describe("standalone review", () => {
  test("loads bounded review data with exact changeset identities", async () => {
    const commands: LocalCommand[] = [];
    const review = await loadStandaloneReview(
      createClient(commands),
      CHANGESET_ID
    );

    expect(commands).toEqual([
      { type: "get_diff", data: { changeset_id: CHANGESET_ID } },
      { type: "get_findings", data: { changeset_id: CHANGESET_ID } },
      {
        type: "get_history",
        data: { changeset_id: CHANGESET_ID, limit: 100 },
      },
      { type: "get_recovery" },
    ]);
    expect(review.diff.unified_diff).toBe("diff");
  });

  test("rejects review data for another changeset", async () => {
    const commands: LocalCommand[] = [];
    const client = createClient(commands, (command) => {
      const response = responseData(command) as {
        data?: { changeset_id?: string };
        type: string;
      };
      if (command.type === "get_diff" && response.data) {
        response.data.changeset_id = "other-changeset";
      }
      return response;
    });

    try {
      await loadStandaloneReview(client, CHANGESET_ID);
      throw new Error("Expected mismatched review data to be rejected.");
    } catch (error) {
      expect(error).toEqual(
        expect.objectContaining({ code: "review_mismatch" })
      );
    }
  });

  test("runs typed checks against the exact changeset version and head", async () => {
    const commands: LocalCommand[] = [];
    const report = await runStandaloneReview(
      createClient(commands),
      changeset(),
      reviewChecks
    );

    expect(commands).toEqual([
      {
        type: "review_changeset",
        data: {
          changeset_id: CHANGESET_ID,
          expected_version: 4,
          expected_head_sha: "head-sha",
          checks: reviewChecks,
        },
      },
    ]);
    expect(report.checks.map((check) => check.kind)).toEqual(reviewChecks);
  });

  test("rejects a review result for another changeset head", async () => {
    const commands: LocalCommand[] = [];
    const client = createClient(commands, (command) => {
      const response = responseData(command);
      if (command.type !== "review_changeset") {
        return response;
      }
      return {
        type: "review_completed",
        data: { ...reviewReport(), head_sha: "other-head" },
      };
    });

    try {
      await runStandaloneReview(client, changeset(), reviewChecks);
      throw new Error("Expected mismatched review result to be rejected.");
    } catch (error) {
      expect(error).toEqual(
        expect.objectContaining({ code: "review_result_mismatch" })
      );
    }
  });

  test("previews the exact version and head", async () => {
    const commands: LocalCommand[] = [];
    const preview = await previewChangesetMutation(
      createClient(commands),
      changeset(),
      "discard"
    );

    expect(commands).toEqual([
      {
        type: "preview_discard",
        data: {
          changeset_id: CHANGESET_ID,
          expected_version: 4,
          expected_head_sha: "head-sha",
        },
      },
    ]);
    expect(preview).toEqual(mutationPreview("discard"));
  });

  test("rejects a preview that does not match the changeset", async () => {
    const commands: LocalCommand[] = [];
    const client = createClient(commands, (command) => {
      const response = responseData(command);
      if (command.type !== "preview_commit") {
        return response;
      }
      return {
        type: "mutation_preview",
        data: { ...mutationPreview("commit"), expected_version: 3 },
      };
    });

    try {
      await previewChangesetMutation(client, changeset(), "commit");
      throw new Error("Expected mismatched preview to be rejected.");
    } catch (error) {
      expect(error).toEqual(
        expect.objectContaining({ code: "mutation_preview_mismatch" })
      );
    }
  });

  test("submits the exact confirmation and validates both result kinds", async () => {
    const commands: LocalCommand[] = [];
    const client = createClient(commands);

    const commitResult = await completeChangesetMutation(
      client,
      mutationPreview("commit")
    );
    const discardResult = await completeChangesetMutation(
      client,
      mutationPreview("discard")
    );

    expect(commands).toEqual([
      {
        type: "commit_changeset",
        data: {
          changeset_id: CHANGESET_ID,
          expected_version: 4,
          expected_head_sha: "head-sha",
          confirmation_digest: "commit-confirmation-digest",
        },
      },
      {
        type: "discard_changeset",
        data: {
          changeset_id: CHANGESET_ID,
          expected_version: 4,
          expected_head_sha: "head-sha",
          confirmation_digest: "discard-confirmation-digest",
        },
      },
    ]);
    expect(commitResult.kind).toBe("commit");
    expect(discardResult.kind).toBe("discard");
  });

  test("rejects an internally inconsistent terminal result", async () => {
    const commands: LocalCommand[] = [];
    const client = createClient(commands, (command) => {
      const response = responseData(command);
      if (command.type !== "commit_changeset") {
        return response;
      }
      return {
        type: "mutation_completed",
        data: {
          ...mutationResult("commit"),
          resulting_head_sha: "different-head-sha",
        },
      };
    });

    try {
      await completeChangesetMutation(client, mutationPreview("commit"));
      throw new Error("Expected inconsistent terminal result to be rejected.");
    } catch (error) {
      expect(error).toEqual(
        expect.objectContaining({ code: "mutation_result_mismatch" })
      );
    }
  });

  test("rejects a result that does not match the confirmed preview", async () => {
    const commands: LocalCommand[] = [];
    const client = createClient(commands, (command) => {
      const response = responseData(command);
      if (command.type !== "commit_changeset") {
        return response;
      }
      return {
        type: "mutation_completed",
        data: {
          ...mutationResult("commit"),
          manifest_sha256: "other-manifest",
        },
      };
    });

    try {
      await completeChangesetMutation(client, mutationPreview("commit"));
      throw new Error("Expected mismatched result to be rejected.");
    } catch (error) {
      expect(error).toEqual(
        expect.objectContaining({ code: "mutation_result_mismatch" })
      );
    }
  });
});
