import type {
  ActionProposal,
  OrderedRunEvent,
  RunState,
} from "@workspace/shared/protocol";

const MAX_RETAINED_EVENTS = 1000;
const MAX_RETAINED_TEXT_BYTES = 1024 * 1024;
const textEncoder = new TextEncoder();

export type PendingApproval = {
  digest: string;
  proposal: ActionProposal;
};

export type ExecutionEventState = {
  events: readonly OrderedRunEvent[];
  hasGap: boolean;
  lastSequence: number;
  pendingApproval: PendingApproval | null;
  runId: string | null;
  runState: RunState | null;
};

export const initialExecutionEventState = (): ExecutionEventState => ({
  events: [],
  hasGap: false,
  lastSequence: 0,
  pendingApproval: null,
  runId: null,
  runState: null,
});

const retainedTextBytes = (events: readonly OrderedRunEvent[]): number => {
  let bytes = 0;
  for (const ordered of events) {
    if (ordered.event.type === "text") {
      bytes += textEncoder.encode(ordered.event.data.text).byteLength;
    }
  }
  return bytes;
};

const retainBoundedEvents = (
  previous: readonly OrderedRunEvent[],
  ordered: OrderedRunEvent
): readonly OrderedRunEvent[] => {
  const events = [...previous, ordered];
  let textBytes = retainedTextBytes(events);

  while (
    events.length > MAX_RETAINED_EVENTS ||
    textBytes > MAX_RETAINED_TEXT_BYTES
  ) {
    const removed = events.shift();
    if (removed?.event.type === "text") {
      textBytes -= textEncoder.encode(removed.event.data.text).byteLength;
    }
  }

  return events;
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

  if (ordered.event.type === "action_proposal") {
    pendingApproval = ordered.event.data;
  } else if (
    ordered.event.type === "approval" &&
    pendingApproval?.digest === ordered.event.data.digest
  ) {
    pendingApproval = null;
  } else if (ordered.event.type === "lifecycle") {
    runState = ordered.event.data.state;
  }

  return {
    events: retainBoundedEvents(state.events, ordered),
    hasGap: false,
    lastSequence: ordered.sequence,
    pendingApproval,
    runId: state.runId ?? ordered.run_id,
    runState,
  };
};
