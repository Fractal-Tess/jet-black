import type {
  ApprovalRequest,
  OrderedRunEvent,
  RunSnapshot,
  RunState,
} from "@workspace/shared/protocol";

const MAX_RETAINED_EVENTS = 1000;
const MAX_RETAINED_TEXT_BYTES = 1024 * 1024;
const textEncoder = new TextEncoder();

export type ExecutionEventState = {
  events: readonly OrderedRunEvent[];
  hasGap: boolean;
  lastSequence: number;
  pendingApproval: ApprovalRequest | null;
  retainedTextBytes: number;
  runId: string | null;
  runState: RunState | null;
};

export const initialExecutionEventState = (): ExecutionEventState => ({
  events: [],
  hasGap: false,
  lastSequence: 0,
  pendingApproval: null,
  retainedTextBytes: 0,
  runId: null,
  runState: null,
});

const eventTextBytes = (ordered: OrderedRunEvent): number =>
  ordered.event.type === "text"
    ? textEncoder.encode(ordered.event.data.text).byteLength
    : 0;

const retainBoundedEvents = (
  previous: readonly OrderedRunEvent[],
  previousTextBytes: number,
  ordered: OrderedRunEvent
): { events: readonly OrderedRunEvent[]; textBytes: number } => {
  const events = [...previous, ordered];
  let textBytes = previousTextBytes + eventTextBytes(ordered);

  while (
    events.length > MAX_RETAINED_EVENTS ||
    textBytes > MAX_RETAINED_TEXT_BYTES
  ) {
    const removed = events.shift();
    if (removed !== undefined) {
      textBytes -= eventTextBytes(removed);
    }
  }

  return { events, textBytes };
};

export const reduceExecutionEvent = (
  state: ExecutionEventState,
  ordered: OrderedRunEvent
): ExecutionEventState => {
  if (
    ordered.sequence <= state.lastSequence ||
    (state.runId !== null && state.runId !== ordered.run_id)
  ) {
    return state;
  }

  if (ordered.sequence !== state.lastSequence + 1) {
    if (state.hasGap && state.runId !== null) {
      return state;
    }

    return {
      ...state,
      hasGap: true,
      runId: state.runId ?? ordered.run_id,
    };
  }

  let pendingApproval = state.pendingApproval;
  let runState = state.runState;

  if (
    ordered.event.type === "action_proposal" &&
    pendingApproval?.digest !== ordered.event.data.digest
  ) {
    pendingApproval = null;
  } else if (
    ordered.event.type === "approval" &&
    pendingApproval?.digest === ordered.event.data.digest
  ) {
    pendingApproval = null;
  } else if (ordered.event.type === "lifecycle") {
    runState = ordered.event.data.state;
  }

  const retained = retainBoundedEvents(
    state.events,
    state.retainedTextBytes,
    ordered
  );
  return {
    events: retained.events,
    hasGap: false,
    lastSequence: ordered.sequence,
    pendingApproval,
    retainedTextBytes: retained.textBytes,
    runId: state.runId ?? ordered.run_id,
    runState,
  };
};

export const executionEventStateFromSnapshot = (
  snapshot: RunSnapshot
): ExecutionEventState => {
  let state: ExecutionEventState = {
    ...initialExecutionEventState(),
    runId: snapshot.run.id,
    runState: snapshot.run.state,
  };

  for (const event of snapshot.events) {
    state = reduceExecutionEvent(state, event);
  }

  return {
    ...state,
    pendingApproval: snapshot.pending_approval,
    runState: snapshot.run.state,
  };
};

export const hydrateExecutionSnapshot = (
  state: ExecutionEventState,
  snapshot: RunSnapshot
): ExecutionEventState => {
  if (state.runId !== null && state.runId !== snapshot.run.id) {
    return state;
  }

  return {
    ...state,
    pendingApproval: snapshot.pending_approval,
    runId: snapshot.run.id,
    runState: snapshot.run.state,
  };
};
