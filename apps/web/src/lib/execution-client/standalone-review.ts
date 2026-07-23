import type {
  Changeset,
  DiffResponse,
  FindingsResponse,
  HistoryResponse,
  MutationPreview,
  MutationResult,
  RecoveryResponse,
  ReviewCheckKind,
  ReviewReport,
} from "@workspace/shared/protocol";
import { ExecutionClientError, unwrapCommandResult } from "./errors";
import type { ExecutionClient } from "./types";

const REVIEW_HISTORY_LIMIT = 100;

export type StandaloneReviewData = {
  diff: DiffResponse;
  findings: FindingsResponse;
  history: HistoryResponse;
  recovery: RecoveryResponse;
};

export const loadStandaloneReview = async (
  client: ExecutionClient,
  changesetId: string
): Promise<StandaloneReviewData> => {
  const [diffEnvelope, findingsEnvelope, historyEnvelope, recoveryEnvelope] =
    await Promise.all([
      client.command({ type: "get_diff", data: { changeset_id: changesetId } }),
      client.command({
        type: "get_findings",
        data: { changeset_id: changesetId },
      }),
      client.command({
        type: "get_history",
        data: { changeset_id: changesetId, limit: REVIEW_HISTORY_LIMIT },
      }),
      client.command({ type: "get_recovery" }),
    ]);
  const review = {
    diff: unwrapCommandResult(diffEnvelope).data,
    findings: unwrapCommandResult(findingsEnvelope).data,
    history: unwrapCommandResult(historyEnvelope).data,
    recovery: unwrapCommandResult(recoveryEnvelope).data,
  };

  if (
    review.diff.changeset_id !== changesetId ||
    review.findings.changeset_id !== changesetId ||
    review.history.changeset_id !== changesetId
  ) {
    throw new ExecutionClientError({
      code: "review_mismatch",
      message: "Review data belongs to a different changeset.",
      retryable: false,
    });
  }

  return review;
};

const changesetRevision = (changeset: Changeset) => ({
  changeset_id: changeset.id,
  expected_version: changeset.version,
  expected_head_sha: changeset.head_sha,
});

const assertChangesetRevisionMatches = (
  changesetId: string,
  changesetVersion: number,
  headSha: string,
  changeset: Changeset,
  error: { code: string; message: string }
): void => {
  if (
    changesetId !== changeset.id ||
    changesetVersion !== changeset.version ||
    headSha !== changeset.head_sha
  ) {
    throw new ExecutionClientError({ ...error, retryable: false });
  }
};

export const runStandaloneReview = async (
  client: ExecutionClient,
  changeset: Changeset,
  checks: readonly ReviewCheckKind[]
): Promise<ReviewReport> => {
  const response = await client.command({
    type: "review_changeset",
    data: { ...changesetRevision(changeset), checks: [...checks] },
  });
  const report = unwrapCommandResult(response).data;
  assertChangesetRevisionMatches(
    report.changeset_id,
    report.changeset_version,
    report.head_sha,
    changeset,
    {
      code: "review_result_mismatch",
      message: "Review result does not match the requested changeset.",
    }
  );
  return report;
};

const assertPreviewMatches = (
  preview: MutationPreview,
  changeset: Changeset,
  kind: MutationPreview["kind"]
): MutationPreview => {
  if (preview.kind !== kind) {
    throw new ExecutionClientError({
      code: "mutation_preview_mismatch",
      message: "Finalization preview does not match the current changeset.",
      retryable: false,
    });
  }
  assertChangesetRevisionMatches(
    preview.changeset_id,
    preview.expected_version,
    preview.expected_head_sha,
    changeset,
    {
      code: "mutation_preview_mismatch",
      message: "Finalization preview does not match the current changeset.",
    }
  );
  return preview;
};

export const previewChangesetMutation = async (
  client: ExecutionClient,
  changeset: Changeset,
  kind: MutationPreview["kind"]
): Promise<MutationPreview> => {
  const data = changesetRevision(changeset);
  const response =
    kind === "commit"
      ? await client.command({ type: "preview_commit", data })
      : await client.command({ type: "preview_discard", data });
  return assertPreviewMatches(
    unwrapCommandResult(response).data,
    changeset,
    kind
  );
};

const assertResultMatches = (
  result: MutationResult,
  preview: MutationPreview
): MutationResult => {
  const expectedState = preview.kind === "commit" ? "committed" : "discarded";
  const commitHeadMismatch =
    result.kind === "commit" &&
    result.changeset.head_sha !== result.resulting_head_sha;
  if (
    result.kind !== preview.kind ||
    result.changeset.id !== preview.changeset_id ||
    result.changeset.state !== expectedState ||
    result.changeset.version <= preview.expected_version ||
    result.worktree_state !== "removed" ||
    result.confirmation_digest !== preview.confirmation_digest ||
    result.checkpoint_id !== preview.checkpoint_id ||
    result.manifest_sha256 !== preview.manifest_sha256 ||
    commitHeadMismatch
  ) {
    throw new ExecutionClientError({
      code: "mutation_result_mismatch",
      message: "Finalization result does not match the confirmed preview.",
      retryable: false,
    });
  }
  return result;
};

export const completeChangesetMutation = async (
  client: ExecutionClient,
  preview: MutationPreview
): Promise<MutationResult> => {
  const data = {
    changeset_id: preview.changeset_id,
    expected_version: preview.expected_version,
    expected_head_sha: preview.expected_head_sha,
    confirmation_digest: preview.confirmation_digest,
  };
  if (preview.kind === "commit") {
    const response = await client.command({ type: "commit_changeset", data });
    return assertResultMatches(unwrapCommandResult(response).data, preview);
  }

  const response = await client.command({ type: "discard_changeset", data });
  return assertResultMatches(unwrapCommandResult(response).data, preview);
};
