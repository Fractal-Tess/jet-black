import type {
  Changeset,
  Repository,
  RunStartedResponse,
} from "@workspace/shared/protocol";
import { ExecutionClientError, unwrapCommandResult } from "./errors";
import type { ExecutionClient } from "./types";

export type StandaloneRunSetup = {
  changeset: Changeset;
  repository: Repository;
  run: RunStartedResponse;
};

export const startStandaloneRun = async (
  client: ExecutionClient,
  repositoryPath: string
): Promise<StandaloneRunSetup> => {
  if (repositoryPath.trim().length === 0) {
    throw new ExecutionClientError({
      code: "invalid_repository_path",
      message: "Enter an approved repository path.",
      retryable: false,
    });
  }

  const repositoryResponse = unwrapCommandResult(
    await client.command({
      type: "register_repository",
      data: { path: repositoryPath },
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
    changeset,
    repository,
    run: runResponse.data,
  };
};
