import type {
  ApprovalRequest,
  EventPage,
  OrderedRunEvent,
  RunSnapshot,
  RunState,
} from "@workspace/shared/protocol";
import { z } from "zod";
import type { BrowserCommandSessionOptions } from "./browser-command-session";
import { createBrowserCommandSession } from "./browser-command-session";
import {
  ExecutionClientError,
  isStructuredError,
  unwrapCommandResult,
} from "./errors";
import {
  type ExecutionEventState,
  executionEventStateFromSnapshot,
  hydrateExecutionSnapshot,
  reduceExecutionEvent,
} from "./event-reducer";
import type { ExecutionClient } from "./types";

const EVENT_PAGE_SIZE = 1000;
const MAX_REPLAY_PAGES = 100;
const TERMINAL_RUN_STATES = new Set<RunState>([
  "completed",
  "failed",
  "interrupted",
]);

export const isTerminalRunState = (state: RunState | null): boolean =>
  state !== null && TERMINAL_RUN_STATES.has(state);

export type BrowserRunSessionStatus =
  | "connecting"
  | "connected"
  | "recovering"
  | "closed";

export type BrowserRunSessionState = {
  error: ExecutionClientError | null;
  events: ExecutionEventState;
  snapshot: RunSnapshot;
  status: BrowserRunSessionStatus;
};

type EventListener = (event: Event) => void;

export type EventSourceTransport = {
  addEventListener(type: "error" | "message", listener: EventListener): void;
  close(): void;
};

export type EventSourceFactory = (url: string) => EventSourceTransport;

export type BrowserRunSessionOptions = BrowserCommandSessionOptions & {
  createEventSource?: EventSourceFactory;
  eventEndpoint?: (runId: string, afterSequence: number) => string;
  runId: string;
};

type StateListener = (state: BrowserRunSessionState) => void;

const runStateSchema = z.enum([
  "queued",
  "starting",
  "running",
  "awaiting_approval",
  "reviewable",
  "completed",
  "interrupted",
  "failed",
]);

const actionProposalSchema = z.object({
  action: z.literal("write_file"),
  target_path: z.string(),
  content_sha256: z.string(),
});

const semanticEventSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("text"), data: z.object({ text: z.string() }) }),
  z.object({
    type: z.literal("action_proposal"),
    data: z.object({ proposal: actionProposalSchema, digest: z.string() }),
  }),
  z.object({
    type: z.literal("action_result"),
    data: z.object({ digest: z.string(), success: z.boolean() }),
  }),
  z.object({
    type: z.literal("file_change"),
    data: z.object({ path: z.string() }),
  }),
  z.object({
    type: z.literal("approval"),
    data: z.object({ digest: z.string(), approved: z.boolean() }),
  }),
  z.object({
    type: z.literal("lifecycle"),
    data: z.object({ state: runStateSchema }),
  }),
]);

const orderedRunEventSchema = z.object({
  run_id: z.string(),
  sequence: z.int().positive(),
  event: semanticEventSchema,
});

const parseOrderedRunEvent = (value: unknown): OrderedRunEvent => {
  const result = orderedRunEventSchema.safeParse(value);
  if (!result.success) {
    throw new ExecutionClientError({
      code: "invalid_event",
      message: "Execution service returned an invalid run event.",
      retryable: false,
    });
  }

  return result.data;
};

const normalizeError = (error: unknown): ExecutionClientError => {
  if (error instanceof ExecutionClientError) {
    return error;
  }

  return new ExecutionClientError({
    code: "browser_session_error",
    message:
      error instanceof Error ? error.message : "Browser run session failed.",
    retryable: false,
  });
};

export class BrowserRunSession {
  readonly #client: ExecutionClient;
  readonly #createEventSource: EventSourceFactory;
  readonly #eventEndpoint: (runId: string, afterSequence: number) => string;
  readonly #listeners = new Set<StateListener>();
  readonly #runId: string;
  #eventProcessing: Promise<void> = Promise.resolve();
  #eventSource: EventSourceTransport | null = null;
  #eventSourceGeneration = 0;
  #mutationInFlight = false;
  #state: BrowserRunSessionState;

  private constructor({
    client,
    createEventSource,
    eventEndpoint,
    runId,
    snapshot,
  }: {
    client: ExecutionClient;
    createEventSource: EventSourceFactory;
    eventEndpoint: (runId: string, afterSequence: number) => string;
    runId: string;
    snapshot: RunSnapshot;
  }) {
    this.#client = client;
    this.#createEventSource = createEventSource;
    this.#eventEndpoint = eventEndpoint;
    this.#runId = runId;
    this.#state = {
      error: null,
      events: executionEventStateFromSnapshot(snapshot),
      snapshot,
      status: "connecting",
    };
  }

  static async create(
    options: BrowserRunSessionOptions
  ): Promise<BrowserRunSession> {
    const client = await createBrowserCommandSession(options);
    const snapshot = await BrowserRunSession.fetchSnapshot(
      client,
      options.runId
    );
    const runSession = new BrowserRunSession({
      client,
      createEventSource:
        options.createEventSource ?? ((url) => new EventSource(url)),
      eventEndpoint:
        options.eventEndpoint ??
        ((runId, afterSequence) =>
          `/api/runs/${encodeURIComponent(runId)}/events?after_sequence=${afterSequence}`),
      runId: options.runId,
      snapshot,
    });
    runSession.assertSnapshotIntegrity(snapshot);
    await runSession.start();
    return runSession;
  }

  get state(): BrowserRunSessionState {
    return this.#state;
  }

  subscribe(listener: StateListener): () => void {
    listener(this.#state);
    if (this.isClosed()) {
      return () => undefined;
    }

    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  }

  async approve(): Promise<void> {
    await this.respondToApproval(true);
  }

  async reject(): Promise<void> {
    await this.respondToApproval(false);
  }

  async interrupt(): Promise<void> {
    await this.runMutation(async () => {
      unwrapCommandResult(
        await this.#client.command({
          type: "interrupt_run",
          data: { run_id: this.#runId },
        })
      );
      await this.refreshSnapshot();
    });
  }

  private isClosed(): boolean {
    return this.#state.status === "closed";
  }

  close(): void {
    this.closeEventSource();
    if (this.#state.status !== "closed") {
      this.updateState({ ...this.#state, status: "closed" });
    }
    this.#listeners.clear();
  }

  private static async fetchSnapshot(
    client: ExecutionClient,
    runId: string
  ): Promise<RunSnapshot> {
    const response = unwrapCommandResult(
      await client.command({ type: "get_snapshot", data: { run_id: runId } })
    );
    return response.data;
  }

  private async start(): Promise<void> {
    if (isTerminalRunState(this.#state.events.runState)) {
      this.updateState({ ...this.#state, status: "recovering" });
      await this.replayEvents();
      this.close();
      return;
    }

    this.connect();
  }

  private assertSnapshotIntegrity(
    snapshot: RunSnapshot,
    proposalDigest?: string
  ): void {
    const snapshotEventsAreContiguous = snapshot.events.every(
      (event, index) =>
        event.run_id === this.#runId && event.sequence === index + 1
    );
    if (
      snapshot.run.id !== this.#runId ||
      snapshot.run.changeset_id !== snapshot.changeset.id ||
      snapshot.changeset.repository_id !== snapshot.repository.id ||
      !snapshotEventsAreContiguous
    ) {
      throw new ExecutionClientError({
        code: "snapshot_mismatch",
        message: "Execution snapshot contains inconsistent run state.",
        retryable: false,
      });
    }

    const approval = snapshot.pending_approval;
    if (
      approval !== null &&
      (approval.run_id !== this.#runId ||
        approval.scope.repository_id !== snapshot.repository.id ||
        approval.scope.changeset_id !== snapshot.changeset.id ||
        approval.scope.base_sha !== snapshot.changeset.base_sha ||
        approval.scope.head_sha !== snapshot.changeset.head_sha ||
        snapshot.run.proposal_digest !== approval.digest)
    ) {
      throw new ExecutionClientError({
        code: "approval_mismatch",
        message: "Pending approval belongs to a different run.",
        retryable: false,
      });
    }
    if (
      proposalDigest !== undefined &&
      snapshot.run.state === "awaiting_approval" &&
      approval?.digest !== proposalDigest
    ) {
      throw new ExecutionClientError({
        code: "approval_mismatch",
        message: "Pending approval does not match the proposed action.",
        retryable: false,
      });
    }
  }

  private connect(): void {
    if (isTerminalRunState(this.#state.events.runState)) {
      this.close();
      return;
    }

    const generation = this.#eventSourceGeneration + 1;
    this.#eventSourceGeneration = generation;
    const source = this.#createEventSource(
      this.#eventEndpoint(this.#runId, this.#state.events.lastSequence)
    );
    this.#eventSource = source;
    source.addEventListener("message", (event) => {
      const data = "data" in event ? event.data : undefined;
      this.enqueueEvent(async () => {
        if (!this.isActiveSource(source, generation)) {
          return;
        }
        if (typeof data !== "string") {
          throw new ExecutionClientError({
            code: "invalid_event",
            message: "Execution event did not contain text data.",
            retryable: false,
          });
        }
        if (this.#state.error?.code === "transport_error") {
          this.updateState({ ...this.#state, error: null });
        }
        await this.processEvent(parseOrderedRunEvent(JSON.parse(data)));
      });
    });
    source.addEventListener("error", (event) => {
      const data = "data" in event ? event.data : undefined;
      if (!this.isActiveSource(source, generation)) {
        return;
      }
      if (typeof data !== "string" || data.length === 0) {
        this.updateState({
          ...this.#state,
          error: new ExecutionClientError({
            code: "transport_error",
            message: "Execution event stream disconnected and is reconnecting.",
            retryable: true,
          }),
        });
        return;
      }

      this.enqueueEvent(() => {
        if (!this.isActiveSource(source, generation)) {
          return;
        }
        let parsed: unknown;
        try {
          parsed = JSON.parse(data);
        } catch {
          throw new ExecutionClientError({
            code: "event_stream_error",
            message: data,
            retryable: false,
          });
        }
        throw new ExecutionClientError(
          isStructuredError(parsed)
            ? parsed
            : {
                code: "event_stream_error",
                message: "Execution event stream returned an invalid error.",
                retryable: false,
              }
        );
      });
    });
    this.updateState({ ...this.#state, status: "connected" });
  }

  private isActiveSource(
    source: EventSourceTransport,
    generation: number
  ): boolean {
    return (
      this.#eventSource === source && this.#eventSourceGeneration === generation
    );
  }

  private closeEventSource(): void {
    this.#eventSourceGeneration += 1;
    this.#eventSource?.close();
    this.#eventSource = null;
  }

  private enqueueEvent(operation: () => Promise<void> | void): void {
    this.#eventProcessing = this.processAfter(this.#eventProcessing, operation);
  }

  private async processAfter(
    previous: Promise<void>,
    operation: () => Promise<void> | void
  ): Promise<void> {
    try {
      await previous;
      if (this.#state.status !== "closed") {
        await operation();
      }
    } catch (error) {
      this.fail(normalizeError(error));
    }
  }

  private async processEvent(event: OrderedRunEvent): Promise<void> {
    const nextEvents = reduceExecutionEvent(this.#state.events, event);
    if (nextEvents === this.#state.events) {
      return;
    }

    this.updateEvents(nextEvents);
    if (nextEvents.hasGap) {
      await this.recoverGap(event.sequence);
      return;
    }

    if (event.event.type === "action_proposal") {
      await this.refreshSnapshot(event.event.data.digest);
    }
    if (isTerminalRunState(nextEvents.runState)) {
      await this.replayEvents();
      this.close();
    }
  }

  private async recoverGap(targetSequence: number): Promise<void> {
    this.closeEventSource();
    this.updateState({ ...this.#state, status: "recovering" });
    await this.replayEvents(targetSequence);
    await this.refreshSnapshot();
    if (!isTerminalRunState(this.#state.events.runState)) {
      this.connect();
    }
  }

  private async replayEvents(targetSequence?: number): Promise<void> {
    for (let page = 0; page < MAX_REPLAY_PAGES; page += 1) {
      if (this.isClosed()) {
        return;
      }
      const previousSequence = this.#state.events.lastSequence;
      const eventPage = await this.fetchEventPage(previousSequence);
      if (this.isClosed()) {
        return;
      }
      if (eventPage.events.length === 0) {
        if (targetSequence !== undefined && previousSequence < targetSequence) {
          throw new ExecutionClientError({
            code: "event_cursor_gap",
            message: "Execution event replay could not fill the cursor gap.",
            retryable: true,
          });
        }
        return;
      }

      this.applyEventPage(eventPage, previousSequence);
      if (
        targetSequence !== undefined &&
        this.#state.events.lastSequence >= targetSequence
      ) {
        return;
      }
    }

    throw new ExecutionClientError({
      code: "event_replay_limit",
      message: "Execution event replay exceeded the browser recovery limit.",
      retryable: true,
    });
  }

  private async fetchEventPage(afterSequence: number): Promise<EventPage> {
    const response = unwrapCommandResult(
      await this.#client.command({
        type: "get_events",
        data: {
          run_id: this.#runId,
          after_sequence: afterSequence,
          limit: EVENT_PAGE_SIZE,
        },
      })
    );
    if (
      response.data.next_cursor.run_id !== this.#runId ||
      (response.data.events.length === 0 &&
        response.data.next_cursor.after_sequence !== afterSequence)
    ) {
      throw new ExecutionClientError({
        code: "event_cursor_gap",
        message: "Execution event replay returned an invalid cursor.",
        retryable: false,
      });
    }
    return response.data;
  }

  private applyEventPage(eventPage: EventPage, previousSequence: number): void {
    let nextEvents = this.#state.events;
    for (const value of eventPage.events) {
      nextEvents = reduceExecutionEvent(
        nextEvents,
        parseOrderedRunEvent(value)
      );
      if (nextEvents.hasGap) {
        throw new ExecutionClientError({
          code: "event_cursor_gap",
          message: "Execution event replay remained non-contiguous.",
          retryable: true,
        });
      }
    }
    if (
      nextEvents.lastSequence === previousSequence ||
      eventPage.next_cursor.after_sequence !== nextEvents.lastSequence
    ) {
      throw new ExecutionClientError({
        code: "event_cursor_gap",
        message: "Execution event replay did not advance the cursor.",
        retryable: true,
      });
    }
    this.updateEvents(nextEvents);
  }

  private async refreshSnapshot(proposalDigest?: string): Promise<void> {
    const snapshot = await BrowserRunSession.fetchSnapshot(
      this.#client,
      this.#runId
    );
    this.assertSnapshotIntegrity(snapshot, proposalDigest);
    const events = hydrateExecutionSnapshot(this.#state.events, snapshot);
    this.updateState({
      ...this.#state,
      events,
      snapshot: this.synchronizeSnapshot(snapshot, events),
    });
    if (isTerminalRunState(snapshot.run.state)) {
      this.closeEventSource();
      this.updateState({ ...this.#state, status: "recovering" });
      await this.replayEvents();
      this.close();
    }
  }

  private updateEvents(events: ExecutionEventState): void {
    this.updateState({
      ...this.#state,
      events,
      snapshot: this.synchronizeSnapshot(this.#state.snapshot, events),
    });
  }

  private synchronizeSnapshot(
    snapshot: RunSnapshot,
    events: ExecutionEventState
  ): RunSnapshot {
    return {
      ...snapshot,
      run: {
        ...snapshot.run,
        state: events.runState ?? snapshot.run.state,
      },
      pending_approval: events.pendingApproval,
      events: [...events.events],
    };
  }

  private async respondToApproval(approved: boolean): Promise<void> {
    const approval: ApprovalRequest | null = this.#state.events.pendingApproval;
    if (approval === null) {
      throw new ExecutionClientError({
        code: "approval_unavailable",
        message: "No exact pending approval is available for this run.",
        retryable: false,
      });
    }

    await this.runMutation(async () => {
      unwrapCommandResult(
        await this.#client.command({
          type: "respond_to_approval",
          data: {
            run_id: approval.run_id,
            scope: approval.scope,
            approved,
          },
        })
      );
      await this.refreshSnapshot();
    });
  }

  private async runMutation(operation: () => Promise<void>): Promise<void> {
    if (this.#mutationInFlight) {
      throw new ExecutionClientError({
        code: "mutation_in_progress",
        message: "Another run mutation is already in progress.",
        retryable: true,
      });
    }

    this.#mutationInFlight = true;
    try {
      await operation();
    } finally {
      this.#mutationInFlight = false;
    }
  }

  private fail(error: ExecutionClientError): void {
    this.closeEventSource();
    this.updateState({ ...this.#state, error, status: "closed" });
    this.#listeners.clear();
  }

  private updateState(state: BrowserRunSessionState): void {
    if (state === this.#state) {
      return;
    }
    this.#state = state;
    for (const listener of this.#listeners) {
      listener(state);
    }
  }
}

export const createBrowserRunSession = (
  options: BrowserRunSessionOptions
): Promise<BrowserRunSession> => BrowserRunSession.create(options);
