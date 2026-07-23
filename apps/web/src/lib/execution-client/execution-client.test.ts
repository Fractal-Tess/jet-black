/// <reference types="bun" />

import { describe, expect, test } from "bun:test";
import {
  type ActionProposal,
  type Envelope,
  type LocalCommand,
  type OrderedRunEvent,
  PROTOCOL_VERSION,
} from "@workspace/shared/protocol";
import { ExecutionClientError, unwrapCommandResult } from "./errors";
import {
  initialExecutionEventState,
  reduceExecutionEvent,
} from "./event-reducer";
import { createExecutionClient } from "./factory";
import { createCommandEnvelope } from "./types";

const getRecoveryCommand = {
  type: "get_recovery",
} as const satisfies LocalCommand;
const HTTP_REQUEST_ID = "123e4567-e89b-42d3-a456-426614174000";
const TAURI_REQUEST_ID = "123e4567-e89b-42d3-a456-426614174001";

describe("execution client factory", () => {
  test("defaults to disabled without requiring transport", async () => {
    const client = createExecutionClient();
    let error: unknown;

    try {
      await client.command(getRecoveryCommand);
    } catch (caught) {
      error = caught;
    }

    expect(client.mode).toBe("disabled");
    expect(error).toBeInstanceOf(ExecutionClientError);
    expect(error).toEqual(
      expect.objectContaining({
        code: "execution_disabled",
        retryable: false,
      })
    );
  });

  test("sends typed envelopes through the HTTP adapter", async () => {
    const requests: Envelope<LocalCommand>[] = [];
    const client = createExecutionClient({
      mode: "http",
      createRequestId: () => HTTP_REQUEST_ID,
      endpoint: "/api/commands",
      fetch: (_input, init) => {
        const request = JSON.parse(
          String(init?.body)
        ) as Envelope<LocalCommand>;
        requests.push(request);
        return Promise.resolve(
          Response.json({
            version: PROTOCOL_VERSION,
            request_id: request.request_id,
            result: {
              status: "ok",
              data: { type: "recovery", data: { actions: [] } },
            },
          })
        );
      },
    });

    const response = await client.command(getRecoveryCommand);

    expect(requests).toEqual([
      {
        version: PROTOCOL_VERSION,
        request_id: HTTP_REQUEST_ID,
        payload: getRecoveryCommand,
      },
    ]);
    expect(unwrapCommandResult(response)).toEqual({
      type: "recovery",
      data: { actions: [] },
    });
  });

  test("rejects request IDs that Rust cannot deserialize", () => {
    expect(() =>
      createCommandEnvelope(getRecoveryCommand, () => "invalid-request-id")
    ).toThrow("Execution request IDs must be valid UUIDs.");
  });

  test("uses one allowlisted Tauri command", async () => {
    let invokedCommand = "";
    let invokedArguments: Record<string, unknown> = {};
    const client = createExecutionClient({
      mode: "tauri",
      createRequestId: () => TAURI_REQUEST_ID,
      invoke: (command, arguments_) => {
        invokedCommand = command;
        invokedArguments = arguments_;
        return Promise.resolve({
          version: PROTOCOL_VERSION,
          request_id: TAURI_REQUEST_ID,
          result: {
            status: "ok",
            data: { type: "recovery", data: { actions: [] } },
          },
        });
      },
    });

    await client.command(getRecoveryCommand);

    expect(invokedCommand).toBe("execution_command");
    expect(invokedArguments).toEqual({
      envelope: {
        version: PROTOCOL_VERSION,
        request_id: TAURI_REQUEST_ID,
        payload: getRecoveryCommand,
      },
    });
  });

  test("normalizes Tauri invocation failures", async () => {
    const client = createExecutionClient({
      mode: "tauri",
      createRequestId: () => TAURI_REQUEST_ID,
      invoke: () => Promise.reject(new Error("invoke failed")),
    });
    let error: unknown;

    try {
      await client.command(getRecoveryCommand);
    } catch (caught) {
      error = caught;
    }

    expect(error).toEqual(
      expect.objectContaining({
        code: "transport_error",
        retryable: true,
      })
    );
  });
});

describe("execution event reducer", () => {
  test("deduplicates replay and records sequence gaps", () => {
    const proposal: ActionProposal = {
      action: "write_file",
      target_path: "approved.txt",
      content_sha256: "digest",
    };
    const first: OrderedRunEvent = {
      run_id: "run-1",
      sequence: 1,
      event: {
        type: "action_proposal",
        data: { proposal, digest: "approval-digest" },
      },
    };
    const second: OrderedRunEvent = {
      run_id: "run-1",
      sequence: 2,
      event: {
        type: "text",
        data: { text: "Preparing approval." },
      },
    };
    const third: OrderedRunEvent = {
      run_id: "run-1",
      sequence: 3,
      event: {
        type: "lifecycle",
        data: { state: "awaiting_approval" },
      },
    };

    const afterFirst = reduceExecutionEvent(
      initialExecutionEventState(),
      first
    );
    expect(reduceExecutionEvent(afterFirst, first)).toBe(afterFirst);

    const afterGap = reduceExecutionEvent(afterFirst, third);
    expect(afterGap.hasGap).toBe(true);
    expect(afterGap.lastSequence).toBe(1);
    expect(afterGap.runState).toBeNull();

    const afterReplay = reduceExecutionEvent(
      reduceExecutionEvent(afterGap, second),
      third
    );
    expect(afterReplay.hasGap).toBe(false);
    expect(afterReplay.lastSequence).toBe(3);
    expect(afterReplay.pendingApproval).toBeNull();
    expect(afterReplay.runState).toBe("awaiting_approval");
  });

  test("bounds retained semantic text", () => {
    let state = initialExecutionEventState();
    const text = "x".repeat(64 * 1024);

    for (const sequence of Array.from(
      { length: 17 },
      (_, index) => index + 1
    )) {
      state = reduceExecutionEvent(state, {
        run_id: "run-1",
        sequence,
        event: { type: "text", data: { text } },
      });
    }

    expect(state.events).toHaveLength(16);
    expect(state.events[0]?.sequence).toBe(2);
    expect(state.lastSequence).toBe(17);
  });
});
