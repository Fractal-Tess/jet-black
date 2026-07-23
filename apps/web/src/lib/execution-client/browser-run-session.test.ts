/// <reference types="bun" />

import { describe, expect, test } from "bun:test";
import {
  type ApprovalRequest,
  type LocalCommand,
  type OrderedRunEvent,
  PROTOCOL_VERSION,
  type RunSnapshot,
} from "@workspace/shared/protocol";
import {
  createBrowserRunSession,
  type EventSourceTransport,
} from "./browser-run-session";

const REQUEST_ID = "123e4567-e89b-42d3-a456-426614174010";
const RUN_ID = "123e4567-e89b-42d3-a456-426614174011";
const CHANGESET_ID = "123e4567-e89b-42d3-a456-426614174012";
const REPOSITORY_ID = "123e4567-e89b-42d3-a456-426614174013";
const CSRF_TOKEN = "csrf-token";
const SESSION_KEY = "jet-black.execution-session";

const approvalRequest = (): ApprovalRequest => ({
  run_id: RUN_ID,
  digest: "approval-digest",
  scope: {
    repository_id: REPOSITORY_ID,
    changeset_id: CHANGESET_ID,
    base_sha: "base-sha",
    head_sha: "head-sha",
    proposal: {
      action: "write_file",
      target_path: "approved.txt",
      content_sha256: "content-digest",
    },
    expires_at_unix_ms: 2000,
  },
});

const snapshot = ({
  events = [],
  pendingApproval = null,
  state = "running",
}: {
  events?: OrderedRunEvent[];
  pendingApproval?: ApprovalRequest | null;
  state?: RunSnapshot["run"]["state"];
} = {}): RunSnapshot => ({
  repository: {
    id: REPOSITORY_ID,
    default_branch: "main",
    base_sha: "base-sha",
    version: 1,
  },
  changeset: {
    id: CHANGESET_ID,
    repository_id: REPOSITORY_ID,
    base_sha: "base-sha",
    head_sha: "head-sha",
    state: "active",
    ticket: null,
    version: 1,
  },
  run: {
    id: RUN_ID,
    changeset_id: CHANGESET_ID,
    state,
    proposal_digest: pendingApproval?.digest ?? null,
    kind: "mutation",
    version: 1,
  },
  worktree: null,
  checkpoint: null,
  pending_approval: pendingApproval,
  findings: [],
  events,
});

class MemoryStorage {
  readonly values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

class FakeEventSource implements EventSourceTransport {
  readonly listeners = new Map<
    "error" | "message",
    ((event: Event) => void)[]
  >();
  closed = false;

  addEventListener(
    type: "error" | "message",
    listener: (event: Event) => void
  ): void {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  close(): void {
    this.closed = true;
  }

  emit(type: "error" | "message", data?: string): void {
    for (const listener of this.listeners.get(type) ?? []) {
      listener({ data } as unknown as Event);
    }
  }
}

const storedSession = (storage: MemoryStorage): void => {
  storage.setItem(
    SESSION_KEY,
    JSON.stringify({
      csrf_token: CSRF_TOKEN,
      expires_at_unix_ms: 2000,
    })
  );
};

const commandResponse = (requestId: string, data: unknown): Response =>
  Response.json({
    version: PROTOCOL_VERSION,
    request_id: requestId,
    result: { status: "ok", data },
  });

const readCommand = (
  init?: RequestInit
): {
  command: LocalCommand;
  requestId: string;
} => {
  const envelope = JSON.parse(String(init?.body)) as {
    payload: LocalCommand;
    request_id: string;
  };
  return { command: envelope.payload, requestId: envelope.request_id };
};

const flushOperations = async (): Promise<void> => {
  for (let index = 0; index < 32; index += 1) {
    await Promise.resolve();
  }
};

describe("browser run session", () => {
  test("removes and exchanges the launch token before opening snapshot-first SSE", async () => {
    const operations: string[] = [];
    const storage = new MemoryStorage();
    const sources: FakeEventSource[] = [];
    const sourceUrls: string[] = [];
    const session = await createBrowserRunSession({
      runId: RUN_ID,
      createRequestId: () => REQUEST_ID,
      location: {
        hash: "#exchange=launch-token",
        pathname: "/",
        search: "",
      },
      history: {
        state: null,
        replaceState: (_data, _unused, url) => {
          operations.push(`replace:${String(url)}`);
        },
      },
      storage,
      now: () => 1000,
      fetch: (input, init) => {
        if (String(input) === "/api/session/exchange") {
          operations.push("exchange");
          expect(JSON.parse(String(init?.body))).toEqual({
            token: "launch-token",
          });
          return Promise.resolve(
            Response.json({
              csrf_token: CSRF_TOKEN,
              expires_at_unix_ms: 2000,
            })
          );
        }

        operations.push("snapshot");
        expect(new Headers(init?.headers).get("x-csrf-token")).toBe(CSRF_TOKEN);
        const { requestId } = readCommand(init);
        return Promise.resolve(
          commandResponse(requestId, { type: "snapshot", data: snapshot() })
        );
      },
      createEventSource: (url) => {
        operations.push("event-source");
        sourceUrls.push(url);
        const source = new FakeEventSource();
        sources.push(source);
        return source;
      },
    });

    expect(operations).toEqual([
      "replace:/",
      "exchange",
      "snapshot",
      "event-source",
    ]);
    expect(sourceUrls).toEqual([`/api/runs/${RUN_ID}/events?after_sequence=0`]);
    expect(storage.getItem(SESSION_KEY)).not.toContain("launch-token");
    expect(session.state.status).toBe("connected");
    session.close();
    expect(sources[0]?.closed).toBe(true);
  });

  test("submits only the exact approval scope from the persisted snapshot", async () => {
    const storage = new MemoryStorage();
    storedSession(storage);
    const exactApproval = approvalRequest();
    let currentSnapshot = snapshot({
      pendingApproval: exactApproval,
      state: "awaiting_approval",
    });
    const commands: LocalCommand[] = [];
    const session = await createBrowserRunSession({
      runId: RUN_ID,
      createRequestId: () => REQUEST_ID,
      location: { hash: "", pathname: "/", search: "" },
      history: { state: null, replaceState: () => undefined },
      storage,
      now: () => 1000,
      fetch: (_input, init) => {
        const { command, requestId } = readCommand(init);
        commands.push(command);
        if (command.type === "get_snapshot") {
          return Promise.resolve(
            commandResponse(requestId, {
              type: "snapshot",
              data: currentSnapshot,
            })
          );
        }
        if (command.type === "get_events") {
          return Promise.resolve(
            commandResponse(requestId, {
              type: "events",
              data: {
                events: [],
                next_cursor: {
                  run_id: RUN_ID,
                  after_sequence: command.data.after_sequence,
                },
              },
            })
          );
        }

        expect(command).toEqual({
          type: "respond_to_approval",
          data: {
            run_id: RUN_ID,
            scope: exactApproval.scope,
            approved: false,
          },
        });
        currentSnapshot = snapshot({ state: "interrupted" });
        return Promise.resolve(
          commandResponse(requestId, {
            type: "approval_rejected",
            data: currentSnapshot.run,
          })
        );
      },
      createEventSource: () => new FakeEventSource(),
    });

    await session.reject();

    expect(commands.map((command) => command.type)).toEqual([
      "get_snapshot",
      "respond_to_approval",
      "get_snapshot",
      "get_events",
    ]);
    expect(session.state.events.pendingApproval).toBeNull();
    expect(session.state.status).toBe("closed");
  });

  test("recovers a cursor gap before reconnecting SSE", async () => {
    const storage = new MemoryStorage();
    storedSession(storage);
    const first: OrderedRunEvent = {
      run_id: RUN_ID,
      sequence: 1,
      event: { type: "text", data: { text: "one" } },
    };
    const second: OrderedRunEvent = {
      run_id: RUN_ID,
      sequence: 2,
      event: { type: "text", data: { text: "two" } },
    };
    const third: OrderedRunEvent = {
      run_id: RUN_ID,
      sequence: 3,
      event: { type: "text", data: { text: "three" } },
    };
    const sources: FakeEventSource[] = [];
    const sourceUrls: string[] = [];
    const replayCursors: number[] = [];
    const session = await createBrowserRunSession({
      runId: RUN_ID,
      createRequestId: () => REQUEST_ID,
      location: { hash: "", pathname: "/", search: "" },
      history: { state: null, replaceState: () => undefined },
      storage,
      now: () => 1000,
      fetch: (_input, init) => {
        const { command, requestId } = readCommand(init);
        if (command.type === "get_snapshot") {
          return Promise.resolve(
            commandResponse(requestId, {
              type: "snapshot",
              data: snapshot({ events: [first] }),
            })
          );
        }
        if (command.type === "get_events") {
          replayCursors.push(command.data.after_sequence);
          return Promise.resolve(
            commandResponse(requestId, {
              type: "events",
              data: {
                events: [second, third],
                next_cursor: { run_id: RUN_ID, after_sequence: 3 },
              },
            })
          );
        }
        throw new Error(`Unexpected command: ${command.type}`);
      },
      createEventSource: (url) => {
        sourceUrls.push(url);
        const source = new FakeEventSource();
        sources.push(source);
        return source;
      },
    });

    sources[0]?.emit("message", JSON.stringify(third));
    sources[0]?.emit(
      "error",
      JSON.stringify({
        code: "stale_stream_error",
        message: "stale source",
        retryable: false,
      })
    );
    await flushOperations();

    expect(replayCursors).toEqual([1]);
    expect(sourceUrls).toEqual([
      `/api/runs/${RUN_ID}/events?after_sequence=1`,
      `/api/runs/${RUN_ID}/events?after_sequence=3`,
    ]);
    expect(session.state.events.events.map((event) => event.sequence)).toEqual([
      1, 2, 3,
    ]);
    expect(session.state.events.hasGap).toBe(false);
    expect(session.state.status).toBe("connected");
    expect(sources[1]?.closed).toBe(false);
    session.close();
  });

  test("replays all remaining events before closing a terminal snapshot", async () => {
    const storage = new MemoryStorage();
    storedSession(storage);
    const first: OrderedRunEvent = {
      run_id: RUN_ID,
      sequence: 1,
      event: { type: "text", data: { text: "one" } },
    };
    const second: OrderedRunEvent = {
      run_id: RUN_ID,
      sequence: 2,
      event: { type: "text", data: { text: "two" } },
    };
    let replayRequests = 0;
    let eventSourceCreated = false;
    const session = await createBrowserRunSession({
      runId: RUN_ID,
      createRequestId: () => REQUEST_ID,
      location: { hash: "", pathname: "/", search: "" },
      history: { state: null, replaceState: () => undefined },
      storage,
      now: () => 1000,
      fetch: (_input, init) => {
        const { command, requestId } = readCommand(init);
        if (command.type === "get_snapshot") {
          return Promise.resolve(
            commandResponse(requestId, {
              type: "snapshot",
              data: snapshot({ events: [first], state: "completed" }),
            })
          );
        }
        if (command.type === "get_events") {
          replayRequests += 1;
          const events = replayRequests === 1 ? [second] : [];
          return Promise.resolve(
            commandResponse(requestId, {
              type: "events",
              data: {
                events,
                next_cursor: {
                  run_id: RUN_ID,
                  after_sequence: events.at(-1)?.sequence ?? 2,
                },
              },
            })
          );
        }
        throw new Error(`Unexpected command: ${command.type}`);
      },
      createEventSource: () => {
        eventSourceCreated = true;
        return new FakeEventSource();
      },
    });

    expect(replayRequests).toBe(2);
    expect(eventSourceCreated).toBe(false);
    expect(session.state.events.events.map((event) => event.sequence)).toEqual([
      1, 2,
    ]);
    expect(
      session.state.snapshot.events.map((event) => event.sequence)
    ).toEqual([1, 2]);
    expect(session.state.status).toBe("closed");
  });

  test("keeps native EventSource reconnects alive", async () => {
    const storage = new MemoryStorage();
    storedSession(storage);
    const source = new FakeEventSource();
    const session = await createBrowserRunSession({
      runId: RUN_ID,
      createRequestId: () => REQUEST_ID,
      location: { hash: "", pathname: "/", search: "" },
      history: { state: null, replaceState: () => undefined },
      storage,
      now: () => 1000,
      fetch: (_input, init) => {
        const { requestId } = readCommand(init);
        return Promise.resolve(
          commandResponse(requestId, { type: "snapshot", data: snapshot() })
        );
      },
      createEventSource: () => source,
    });

    source.emit("error");
    expect(session.state.status).toBe("connected");
    expect(session.state.error?.code).toBe("transport_error");
    expect(source.closed).toBe(false);

    source.emit(
      "message",
      JSON.stringify({
        run_id: RUN_ID,
        sequence: 1,
        event: { type: "text", data: { text: "reconnected" } },
      } satisfies OrderedRunEvent)
    );
    await flushOperations();

    expect(session.state.error).toBeNull();
    expect(session.state.events.lastSequence).toBe(1);
    session.close();
  });

  test("hydrates a new proposal from the exact snapshot approval", async () => {
    const storage = new MemoryStorage();
    storedSession(storage);
    const source = new FakeEventSource();
    const exactApproval = approvalRequest();
    let snapshotCount = 0;
    const session = await createBrowserRunSession({
      runId: RUN_ID,
      createRequestId: () => REQUEST_ID,
      location: { hash: "", pathname: "/", search: "" },
      history: { state: null, replaceState: () => undefined },
      storage,
      now: () => 1000,
      fetch: (_input, init) => {
        const { requestId } = readCommand(init);
        snapshotCount += 1;
        return Promise.resolve(
          commandResponse(requestId, {
            type: "snapshot",
            data:
              snapshotCount === 1
                ? snapshot()
                : snapshot({
                    pendingApproval: exactApproval,
                    state: "awaiting_approval",
                  }),
          })
        );
      },
      createEventSource: () => source,
    });
    const proposal: OrderedRunEvent = {
      run_id: RUN_ID,
      sequence: 1,
      event: {
        type: "action_proposal",
        data: {
          digest: exactApproval.digest,
          proposal: exactApproval.scope.proposal,
        },
      },
    };

    source.emit("message", JSON.stringify(proposal));
    await flushOperations();

    expect(session.state.events.pendingApproval).toEqual(exactApproval);
    expect(session.state.events.runState).toBe("awaiting_approval");
    session.close();
  });

  test("interrupts the exact run and refreshes its terminal snapshot", async () => {
    const storage = new MemoryStorage();
    storedSession(storage);
    let currentSnapshot = snapshot();
    const commands: LocalCommand[] = [];
    const source = new FakeEventSource();
    const session = await createBrowserRunSession({
      runId: RUN_ID,
      createRequestId: () => REQUEST_ID,
      location: { hash: "", pathname: "/", search: "" },
      history: { state: null, replaceState: () => undefined },
      storage,
      now: () => 1000,
      fetch: (_input, init) => {
        const { command, requestId } = readCommand(init);
        commands.push(command);
        if (command.type === "get_snapshot") {
          return Promise.resolve(
            commandResponse(requestId, {
              type: "snapshot",
              data: currentSnapshot,
            })
          );
        }
        if (command.type === "get_events") {
          return Promise.resolve(
            commandResponse(requestId, {
              type: "events",
              data: {
                events: [],
                next_cursor: {
                  run_id: RUN_ID,
                  after_sequence: command.data.after_sequence,
                },
              },
            })
          );
        }

        expect(command).toEqual({
          type: "interrupt_run",
          data: { run_id: RUN_ID },
        });
        currentSnapshot = snapshot({ state: "interrupted" });
        return Promise.resolve(
          commandResponse(requestId, {
            type: "run_interrupted",
            data: currentSnapshot.run,
          })
        );
      },
      createEventSource: () => source,
    });

    await session.interrupt();

    expect(commands.map((command) => command.type)).toEqual([
      "get_snapshot",
      "interrupt_run",
      "get_snapshot",
      "get_events",
    ]);
    expect(source.closed).toBe(true);
    expect(session.state.events.runState).toBe("interrupted");
    expect(session.state.status).toBe("closed");
  });
});
