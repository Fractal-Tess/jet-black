import type { LocalCommand } from "@workspace/shared/protocol";
import { ExecutionClientError, parseResponseEnvelope } from "./errors";
import {
  createCommandEnvelope,
  type ExecutionClient,
  type ExecutionCommandResponse,
  type FetchTransport,
  type RequestIdFactory,
} from "./types";

export type HttpExecutionClientOptions = {
  createRequestId?: RequestIdFactory;
  endpoint: string;
  fetch?: FetchTransport;
};

export const createHttpExecutionClient = ({
  createRequestId,
  endpoint,
  fetch: fetchTransport = globalThis.fetch,
}: HttpExecutionClientOptions): ExecutionClient => ({
  mode: "http",
  command: async <C extends LocalCommand>(command: C) => {
    const envelope = createCommandEnvelope(command, createRequestId);
    let response: Response;

    try {
      response = await fetchTransport(endpoint, {
        body: JSON.stringify(envelope),
        credentials: "same-origin",
        headers: { "content-type": "application/json" },
        method: "POST",
      });
    } catch {
      throw new ExecutionClientError({
        code: "transport_error",
        message: "Unable to reach the local execution service.",
        retryable: true,
      });
    }

    if (!response.ok) {
      throw new ExecutionClientError({
        code: "http_error",
        message: `Execution service returned HTTP ${response.status}.`,
        retryable: response.status >= 500,
      });
    }

    let body: unknown;
    try {
      body = await response.json();
    } catch {
      throw new ExecutionClientError({
        code: "invalid_response",
        message: "Execution service returned invalid JSON.",
        retryable: false,
      });
    }

    return parseResponseEnvelope<ExecutionCommandResponse<C>>(
      body,
      envelope.request_id
    );
  },
});
