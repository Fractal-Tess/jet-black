import type { LocalCommand } from "@workspace/shared/protocol";
import { ExecutionClientError, parseResponseEnvelope } from "./errors";
import {
  createCommandEnvelope,
  type ExecutionClient,
  type ExecutionCommandResponse,
  type RequestIdFactory,
  type TauriInvoke,
} from "./types";

const TAURI_EXECUTION_COMMAND = "execution_command";

export type TauriExecutionClientOptions = {
  createRequestId?: RequestIdFactory;
  invoke: TauriInvoke;
};

export const createTauriExecutionClient = ({
  createRequestId,
  invoke,
}: TauriExecutionClientOptions): ExecutionClient => ({
  mode: "tauri",
  command: async <C extends LocalCommand>(command: C) => {
    const envelope = createCommandEnvelope(command, createRequestId);
    let response: unknown;

    try {
      response = await invoke(TAURI_EXECUTION_COMMAND, { envelope });
    } catch {
      throw new ExecutionClientError({
        code: "transport_error",
        message: "Unable to invoke the local execution service.",
        retryable: true,
      });
    }

    return parseResponseEnvelope<ExecutionCommandResponse<C>>(
      response,
      envelope.request_id
    );
  },
});
