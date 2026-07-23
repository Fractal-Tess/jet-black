import { createDisabledExecutionClient } from "./disabled";
import {
  createHttpExecutionClient,
  type HttpExecutionClientOptions,
} from "./http";
import {
  createTauriExecutionClient,
  type TauriExecutionClientOptions,
} from "./tauri";
import type { ExecutionClient } from "./types";

export type ExecutionClientConfig =
  | { mode?: "disabled" }
  | ({ mode: "http" } & HttpExecutionClientOptions)
  | ({ mode: "tauri" } & TauriExecutionClientOptions);

export const createExecutionClient = (
  config: ExecutionClientConfig = {}
): ExecutionClient => {
  if (config.mode === "http") {
    return createHttpExecutionClient(config);
  }

  if (config.mode === "tauri") {
    return createTauriExecutionClient(config);
  }

  return createDisabledExecutionClient();
};
