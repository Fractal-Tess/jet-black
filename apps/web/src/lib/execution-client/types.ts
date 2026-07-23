import {
  type Envelope,
  type LocalCommand,
  type LocalCommandResponse,
  type MutationResult,
  PROTOCOL_VERSION,
  type ResponseEnvelope,
} from "@workspace/shared/protocol";

const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;

type CommandResponseTypeMap = {
  create_changeset: "changeset_created";
  list_approved_repositories: "approved_repositories";
  delete_run_artifacts: "run_artifacts_deleted";
  get_checkpoint: "checkpoint";
  get_diff: "diff";
  get_events: "events";
  get_findings: "findings";
  get_history: "history";
  get_recovery: "recovery";
  get_run_artifacts: "run_artifacts";
  get_snapshot: "snapshot";
  interrupt_run: "run_interrupted";
  preview_commit: "mutation_preview";
  preview_discard: "mutation_preview";
  read_run_artifact_segment: "run_artifact_segment";
  register_repository: "repository_registered";
  respond_to_approval: "run_completed" | "approval_rejected";
  review_changeset: "review_completed";
  start_run: "run_started";
};

export type ExecutionClientMode = "disabled" | "http" | "tauri";

type MutationCompletedResponse<Kind extends MutationResult["kind"]> = Omit<
  Extract<LocalCommandResponse, { type: "mutation_completed" }>,
  "data"
> & {
  data: Extract<MutationResult, { kind: Kind }>;
};

export type ExecutionCommandResponse<C extends LocalCommand> = C extends {
  type: "commit_changeset";
}
  ? MutationCompletedResponse<"commit">
  : C extends { type: "discard_changeset" }
    ? MutationCompletedResponse<"discard">
    : C extends {
          type: infer Type extends keyof CommandResponseTypeMap;
        }
      ? Extract<LocalCommandResponse, { type: CommandResponseTypeMap[Type] }>
      : never;

export type FetchTransport = (
  input: RequestInfo | URL,
  init?: RequestInit
) => Promise<Response>;

export type RequestIdFactory = () => string;

export type TauriInvoke = (
  command: string,
  arguments_: Record<string, unknown>
) => Promise<unknown>;

export type ExecutionClient = {
  command<C extends LocalCommand>(
    command: C
  ): Promise<ResponseEnvelope<ExecutionCommandResponse<C>>>;
  readonly mode: ExecutionClientMode;
};

export const createCommandEnvelope = <C extends LocalCommand>(
  payload: C,
  createRequestId: RequestIdFactory = () => crypto.randomUUID()
): Envelope<C> => {
  const requestId = createRequestId();
  if (!UUID_PATTERN.test(requestId)) {
    throw new Error("Execution request IDs must be valid UUIDs.");
  }

  return {
    version: PROTOCOL_VERSION,
    request_id: requestId,
    payload,
  };
};
