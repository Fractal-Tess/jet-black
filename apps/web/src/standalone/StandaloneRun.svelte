<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Button } from "@workspace/ui/components/button";
import { onMount } from "svelte";
import {
  type BrowserRunSession,
  type BrowserRunSessionState,
  createBrowserRunSession,
  isTerminalRunState,
} from "../lib/execution-client/browser-run-session";
import type { ExecutionClient } from "../lib/execution-client/types";
import StandaloneReview from "./StandaloneReview.svelte";

type Props = {
  client: ExecutionClient;
  runId: string;
};

type RunOperation = (session: BrowserRunSession) => Promise<void>;

let { client, runId }: Props = $props();
let runSession = $state<BrowserRunSession | null>(null);
let sessionState = $state<BrowserRunSessionState | null>(null);
let mutationBusy = $state(false);
let errorMessage = $state<string | null>(null);
let displayedError = $derived(
  errorMessage ?? sessionState?.error?.message ?? null
);
let disposed = false;
let unsubscribe = (): void => undefined;

const errorText = (error: unknown): string =>
  error instanceof Error ? error.message : "Run session failed.";

const formatTimestamp = (timestamp: number): string =>
  new Date(timestamp).toLocaleString();

const eventText = (
  event: BrowserRunSessionState["events"]["events"][number]
): string => {
  switch (event.event.type) {
    case "text":
      return event.event.data.text;
    case "action_proposal":
      return `Proposed write to ${event.event.data.proposal.target_path}`;
    case "action_result":
      return event.event.data.success
        ? "Applied the approved action"
        : "The approved action failed";
    case "file_change":
      return `Changed ${event.event.data.path}`;
    case "approval":
      return event.event.data.approved
        ? "Approved the proposed action"
        : "Rejected the proposed action";
    case "lifecycle":
      return `Run entered ${event.event.data.state.replaceAll("_", " ")}`;
    default:
      return "Unknown execution event";
  }
};

const connect = async (signal: AbortSignal): Promise<void> => {
  try {
    const session = await createBrowserRunSession({
      runId,
      fetch: (input, init) => globalThis.fetch(input, { ...init, signal }),
    });
    if (disposed) {
      session.close();
      return;
    }
    runSession = session;
    unsubscribe = session.subscribe((state) => {
      sessionState = state;
    });
  } catch (error) {
    if (!disposed) {
      errorMessage = errorText(error);
    }
  }
};

const performMutation = async (operation: RunOperation): Promise<void> => {
  if (
    runSession === null ||
    sessionState?.status !== "connected" ||
    mutationBusy
  ) {
    return;
  }

  mutationBusy = true;
  errorMessage = null;
  try {
    await operation(runSession);
  } catch (error) {
    errorMessage = errorText(error);
  } finally {
    mutationBusy = false;
  }
};

const approve = (): Promise<void> =>
  performMutation((session) => session.approve());
const reject = (): Promise<void> =>
  performMutation((session) => session.reject());
const interrupt = (): Promise<void> =>
  performMutation((session) => session.interrupt());

onMount(() => {
  const controller = new AbortController();
  connect(controller.signal);
  return () => {
    disposed = true;
    controller.abort();
    unsubscribe();
    runSession?.close();
  };
});
</script>

<section class="mt-8 space-y-6 border-t border-border pt-6" aria-labelledby="run-heading">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <p class="text-xs font-medium tracking-wide text-muted-foreground uppercase">
        Active run
      </p>
      <h3 id="run-heading" class="mt-1 font-semibold">Execution timeline</h3>
    </div>
    <div class="flex items-center gap-2">
      <Badge variant="outline">{sessionState?.events.runState ?? "connecting"}</Badge>
      <Badge variant="outline">{sessionState?.status ?? "connecting"}</Badge>
      {#if sessionState?.status === "connected" && !isTerminalRunState(sessionState.events.runState)}
        <Button variant="outline" onclick={interrupt} disabled={mutationBusy}>
          Interrupt
        </Button>
      {/if}
    </div>
  </div>

  {#if displayedError !== null}
    <p class="text-sm text-destructive" role="alert">{displayedError}</p>
  {/if}

  {#if sessionState?.events.pendingApproval}
    <section class="space-y-4 rounded-lg border border-border bg-muted/30 p-4" aria-labelledby="approval-heading">
      <div>
        <p class="text-xs font-medium tracking-wide text-muted-foreground uppercase">
          Approval required
        </p>
        <h4 id="approval-heading" class="mt-1 font-medium">
          Write {sessionState.events.pendingApproval.scope.proposal.target_path}
        </h4>
      </div>
      <dl class="grid gap-3 text-sm sm:grid-cols-2">
        <div>
          <dt class="text-muted-foreground">Base revision</dt>
          <dd class="mt-1 break-all font-mono text-xs">
            {sessionState.events.pendingApproval.scope.base_sha}
          </dd>
        </div>
        <div>
          <dt class="text-muted-foreground">Proposed revision</dt>
          <dd class="mt-1 break-all font-mono text-xs">
            {sessionState.events.pendingApproval.scope.head_sha}
          </dd>
        </div>
        <div class="sm:col-span-2">
          <dt class="text-muted-foreground">Content SHA-256</dt>
          <dd class="mt-1 break-all font-mono text-xs">
            {sessionState.events.pendingApproval.scope.proposal.content_sha256}
          </dd>
        </div>
        <div class="sm:col-span-2">
          <dt class="text-muted-foreground">Approval digest</dt>
          <dd class="mt-1 break-all font-mono text-xs">
            {sessionState.events.pendingApproval.digest}
          </dd>
        </div>
        <div class="sm:col-span-2">
          <dt class="text-muted-foreground">Expires</dt>
          <dd class="mt-1">
            <time datetime={new Date(sessionState.events.pendingApproval.scope.expires_at_unix_ms).toISOString()}>
              {formatTimestamp(sessionState.events.pendingApproval.scope.expires_at_unix_ms)}
            </time>
          </dd>
        </div>
      </dl>
      <div class="flex flex-wrap gap-2">
        <Button
          onclick={approve}
          disabled={mutationBusy || sessionState.status !== "connected"}
        >
          Approve exact action
        </Button>
        <Button
          variant="outline"
          onclick={reject}
          disabled={mutationBusy || sessionState.status !== "connected"}
        >
          Reject
        </Button>
      </div>
    </section>
  {/if}

  <ol
    class="max-h-96 space-y-3 overflow-y-auto rounded-lg border border-border p-4"
    aria-label="Run events"
    aria-live="polite"
    aria-relevant="additions"
    role="log"
  >
    {#if sessionState === null}
      <li class="text-sm text-muted-foreground">Loading the run snapshot…</li>
    {:else if sessionState.events.events.length === 0}
      <li class="text-sm text-muted-foreground">Waiting for execution events…</li>
    {:else}
      {#each sessionState.events.events as event (event.sequence)}
        <li class="grid grid-cols-[auto_1fr] gap-3 text-sm">
          <span class="font-mono text-xs text-muted-foreground">{event.sequence}</span>
          <span class="whitespace-pre-wrap break-words">{eventText(event)}</span>
        </li>
      {/each}
    {/if}
  </ol>

  {#if sessionState?.snapshot.changeset.state === "reviewable"}
    <StandaloneReview client={client} changeset={sessionState.snapshot.changeset} />
  {/if}
</section>
