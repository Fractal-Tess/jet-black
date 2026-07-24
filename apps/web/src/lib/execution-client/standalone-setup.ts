import {
  type ApprovedRepositorySummary,
  type Changeset,
  PROTOCOL_VERSION,
  type ProviderKind,
  type ProviderSelection,
  type RegisteredRepositorySummary,
  type RunStartedResponse,
} from "@workspace/shared/protocol";
import { z } from "zod";
import { ExecutionClientError, unwrapCommandResult } from "./errors";
import type { ExecutionClient, FetchTransport } from "./types";

const providerKindSchema = z.enum([
  "mock",
  "claude-code",
  "codex",
  "opencode",
] satisfies ProviderKind[]);

const providerSelectionSchema = z.object({
  kind: providerKindSchema,
  model: z.string().min(1).nullable(),
});

const standaloneBootstrapSchema = z.object({
  profile: z.literal("standalone"),
  version: z.string().min(1),
  protocol_version: z.literal(PROTOCOL_VERSION),
  enabled_features: z.array(z.string()),
  provider_availability: z.array(providerKindSchema),
  default_provider: providerSelectionSchema,
});

export type StandaloneBootstrap = z.infer<typeof standaloneBootstrapSchema>;

export type StandaloneRunSetup = {
  approvedRepository: ApprovedRepositorySummary;
  changeset: Changeset;
  providerSelection: ProviderSelection;
  repository: RegisteredRepositorySummary;
  run: RunStartedResponse;
};

export const loadStandaloneBootstrap = async (
  fetchTransport: FetchTransport = globalThis.fetch
): Promise<StandaloneBootstrap> => {
  let response: Response;
  try {
    response = await fetchTransport("/api/bootstrap", {
      credentials: "same-origin",
      method: "GET",
    });
  } catch {
    throw new ExecutionClientError({
      code: "transport_error",
      message: "Unable to load local execution capabilities.",
      retryable: true,
    });
  }

  if (!response.ok) {
    throw new ExecutionClientError({
      code: "bootstrap_failed",
      message: `Execution bootstrap returned HTTP ${response.status}.`,
      retryable: response.status >= 500,
    });
  }

  const result = standaloneBootstrapSchema.safeParse(await response.json());
  if (
    !(
      result.success &&
      result.data.provider_availability.includes(
        result.data.default_provider.kind
      )
    )
  ) {
    throw new ExecutionClientError({
      code: "invalid_response",
      message: "Execution service returned invalid provider capabilities.",
      retryable: false,
    });
  }
  return result.data;
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
  approvedRepository: ApprovedRepositorySummary,
  providerSelection: ProviderSelection
): Promise<StandaloneRunSetup> => {
  if (approvedRepository.id.trim().length === 0) {
    throw new ExecutionClientError({
      code: "invalid_repository_selection",
      message: "Select an approved repository.",
      retryable: false,
    });
  }
  if (
    !providerSelectionSchema.safeParse(providerSelection).success ||
    (providerSelection.kind === "mock" && providerSelection.model !== null)
  ) {
    throw new ExecutionClientError({
      code: "invalid_provider_selection",
      message: "Select a valid provider and model.",
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
      data: {
        changeset_id: changeset.id,
        provider_selection: providerSelection,
      },
    })
  );

  return {
    approvedRepository,
    changeset,
    providerSelection,
    repository,
    run: runResponse.data,
  };
};
