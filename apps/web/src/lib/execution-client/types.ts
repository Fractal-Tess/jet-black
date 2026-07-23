import {
  type CheckpointResponse,
  type DiffResponse,
  type Envelope,
  type EventPage,
  type FindingsResponse,
  type HistoryResponse,
  type LocalCommand,
  type MutationPreview,
  PROTOCOL_VERSION,
  type RecoveryResponse,
  type ResponseEnvelope,
  type RunSnapshot,
} from "@workspace/shared/protocol";

const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;

type CommandResponseMap = {
  get_checkpoint: CheckpointResponse;
  get_diff: DiffResponse;
  get_events: EventPage;
  get_findings: FindingsResponse;
  get_history: HistoryResponse;
  get_recovery: RecoveryResponse;
  get_snapshot: RunSnapshot;
  preview_commit: MutationPreview;
  preview_discard: MutationPreview;
};

export type ExecutionClientMode = "disabled" | "http" | "tauri";

export type ExecutionCommandResponse<C extends LocalCommand> = C extends {
  type: infer Type extends keyof CommandResponseMap;
}
  ? CommandResponseMap[Type]
  : unknown;

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
  createRequestId: RequestIdFactory = crypto.randomUUID
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
