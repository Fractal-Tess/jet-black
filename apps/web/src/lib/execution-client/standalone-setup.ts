import type {
  ApprovedRepositorySummary,
  Changeset,
  RegisteredRepositorySummary,
  RunStartedResponse,
} from "@workspace/shared/protocol";
import { ExecutionClientError, unwrapCommandResult } from "./errors";
import type { ExecutionClient } from "./types";

export type StandaloneRunSetup = {
  approvedRepository: ApprovedRepositorySummary;
  changeset: Changeset;
  repository: RegisteredRepositorySummary;
  run: RunStartedResponse;
};

export const listApprovedRepositories = async (
  client: ExecutionClient
): Promise<ApprovedRepositorySummary[]> => {
  const response = unwrapCommandResult(
    await client.command({ type: "list_approved_repositories" })
  );
  return response.data.repositories;
};

export const startStandaloneRun = async (
  client: ExecutionClient,
  approvedRepository: ApprovedRepositorySummary
): Promise<StandaloneRunSetup> => {
  if (approvedRepository.id.trim().length === 0) {
    throw new ExecutionClientError({
      code: "invalid_repository_selection",
      message: "Select an approved repository.",
      retryable: false,
    });
  }

  const repositoryResponse = unwrapCommandResult(
    await client.command({
      type: "register_repository",
      data: { approved_repository_id: approvedRepository.id },
    })
  );
  const repository = repositoryResponse.data;

  const changesetResponse = unwrapCommandResult(
    await client.command({
      type: "create_changeset",
      data: {
        repository_id: repository.id,
        base_sha: repository.base_sha,
        ticket: null,
      },
    })
  );
  const changeset = changesetResponse.data;

  const runResponse = unwrapCommandResult(
    await client.command({
      type: "start_run",
      data: { changeset_id: changeset.id },
    })
  );

  return {
    approvedRepository,
    changeset,
    repository,
    run: runResponse.data,
  };
};
