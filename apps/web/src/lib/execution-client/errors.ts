import {
  PROTOCOL_VERSION,
  type ResponseEnvelope,
  type StructuredError,
} from "@workspace/shared/protocol";

export class ExecutionClientError extends Error {
  readonly code: string;
  readonly retryable: boolean;

  constructor(error: StructuredError) {
    super(error.message);
    this.name = "ExecutionClientError";
    this.code = error.code;
    this.retryable = error.retryable;
  }
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

const isStructuredError = (value: unknown): value is StructuredError =>
  isRecord(value) &&
  typeof value.code === "string" &&
  typeof value.message === "string" &&
  typeof value.retryable === "boolean";

export const parseResponseEnvelope = <T>(
  value: unknown,
  expectedRequestId: string
): ResponseEnvelope<T> => {
  if (
    !isRecord(value) ||
    value.version !== PROTOCOL_VERSION ||
    value.request_id !== expectedRequestId ||
    !isRecord(value.result)
  ) {
    throw new ExecutionClientError({
      code: "invalid_response",
      message: "Execution service returned an invalid response envelope.",
      retryable: false,
    });
  }

  if (value.result.status === "ok" && "data" in value.result) {
    return value as ResponseEnvelope<T>;
  }

  if (value.result.status === "error" && isStructuredError(value.result.data)) {
    return value as ResponseEnvelope<T>;
  }

  throw new ExecutionClientError({
    code: "invalid_response",
    message: "Execution service returned an invalid command result.",
    retryable: false,
  });
};

export const unwrapCommandResult = <T>(response: ResponseEnvelope<T>): T => {
  if (response.result.status === "error") {
    throw new ExecutionClientError(response.result.data);
  }

  return response.result.data;
};
