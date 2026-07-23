import { ExecutionClientError } from "./errors";
import type { ExecutionClient } from "./types";

export const createDisabledExecutionClient = (): ExecutionClient => ({
  mode: "disabled",
  command: () =>
    Promise.reject(
      new ExecutionClientError({
        code: "execution_disabled",
        message: "Local execution is disabled.",
        retryable: false,
      })
    ),
});

export const defaultExecutionClient = createDisabledExecutionClient();
