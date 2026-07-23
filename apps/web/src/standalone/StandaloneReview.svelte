<script lang="ts">
import type {
  Changeset,
  MutationPreview,
  MutationResult,
} from "@workspace/shared/protocol";
import { Badge } from "@workspace/ui/components/badge";
import { Button } from "@workspace/ui/components/button";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { onMount } from "svelte";
import {
  completeChangesetMutation,
  loadStandaloneReview,
  previewChangesetMutation,
  type StandaloneReviewData,
} from "../lib/execution-client/standalone-review";
import type { ExecutionClient } from "../lib/execution-client/types";

type Props = {
  changeset: Changeset;
  client: ExecutionClient;
};

let { changeset, client }: Props = $props();
let review = $state<StandaloneReviewData | null>(null);
let preview = $state<MutationPreview | null>(null);
let result = $state<MutationResult | null>(null);
let confirmation = $state("");
let loading = $state(true);
let busy = $state(false);
let errorMessage = $state<string | null>(null);
let disposed = false;
let confirmationMatches = $derived(
  preview !== null && confirmation === preview.confirmation_digest
);
let displayedChangeset = $derived(result?.changeset ?? changeset);

const errorText = (error: unknown): string =>
  error instanceof Error ? error.message : "Changeset review failed.";

const loadReview = async (): Promise<void> => {
  try {
    const loadedReview = await loadStandaloneReview(client, changeset.id);
    if (!disposed) {
      review = loadedReview;
    }
  } catch (error) {
    if (!disposed) {
      errorMessage = errorText(error);
    }
  } finally {
    if (!disposed) {
      loading = false;
    }
  }
};

const requestPreview = async (kind: MutationPreview["kind"]): Promise<void> => {
  if (busy || result !== null) {
    return;
  }

  busy = true;
  errorMessage = null;
  confirmation = "";
  preview = null;
  try {
    const nextPreview = await previewChangesetMutation(client, changeset, kind);
    if (!disposed) {
      preview = nextPreview;
    }
  } catch (error) {
    if (!disposed) {
      errorMessage = errorText(error);
    }
  } finally {
    if (!disposed) {
      busy = false;
    }
  }
};

const previewCommit = (): Promise<void> => requestPreview("commit");
const previewDiscard = (): Promise<void> => requestPreview("discard");

const finalize = async (): Promise<void> => {
  if (preview === null || !confirmationMatches || busy || result !== null) {
    return;
  }

  busy = true;
  errorMessage = null;
  try {
    const completedResult = await completeChangesetMutation(client, preview);
    if (!disposed) {
      result = completedResult;
    }
  } catch (error) {
    if (!disposed) {
      errorMessage = errorText(error);
    }
  } finally {
    if (!disposed) {
      busy = false;
    }
  }
};

onMount(() => {
  loadReview();
  return () => {
    disposed = true;
  };
});
</script>

<section
  class="mt-8 space-y-6 border-t border-border pt-6"
  aria-busy={loading}
  aria-labelledby="review-heading"
>
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <p class="text-xs font-medium tracking-wide text-muted-foreground uppercase">
        Changeset review
      </p>
      <h3 id="review-heading" class="mt-1 font-semibold">Review and finalize</h3>
    </div>
    <Badge variant="outline">{displayedChangeset.state}</Badge>
  </div>

  {#if errorMessage !== null}
    <p class="text-sm text-destructive" role="alert">{errorMessage}</p>
  {/if}

  {#if loading}
    <p class="text-sm text-muted-foreground" role="status">Loading review data…</p>
  {:else if review !== null}
    <section class="space-y-3" aria-labelledby="diff-heading">
      <h4 id="diff-heading" class="font-medium">Diff</h4>
      <pre class="max-h-96 overflow-auto rounded-lg border border-border bg-muted/30 p-4 text-xs"><code>{review.diff.unified_diff || "No changes."}</code></pre>
    </section>

    <section class="space-y-3" aria-labelledby="findings-heading">
      <div class="flex items-center justify-between gap-3">
        <h4 id="findings-heading" class="font-medium">Findings</h4>
        <Badge variant="outline">{review.findings.findings.length}</Badge>
      </div>
      {#if review.findings.findings.length === 0}
        <p class="text-sm text-muted-foreground">No findings.</p>
      {:else}
        <ul class="space-y-3">
          {#each review.findings.findings as finding (finding.id)}
            <li class="space-y-2 rounded-lg border border-border p-4 text-sm">
              <div class="flex flex-wrap items-center gap-2">
                <Badge variant="outline">{finding.severity}</Badge>
                <Badge variant="outline">{finding.category}</Badge>
                <span class="font-mono text-xs text-muted-foreground">
                  {finding.path ?? "changeset"}
                </span>
              </div>
              <p>{finding.message}</p>
              {#if finding.evidence.length > 0}
                <pre class="overflow-auto whitespace-pre-wrap text-xs text-muted-foreground"><code>{finding.evidence}</code></pre>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <div class="grid gap-6 lg:grid-cols-2">
      <section class="space-y-3" aria-labelledby="history-heading">
        <h4 id="history-heading" class="font-medium">Run history</h4>
        {#if review.history.runs.length === 0}
          <p class="text-sm text-muted-foreground">No prior runs.</p>
        {:else}
          <ul class="space-y-2 text-sm">
            {#each review.history.runs as run (run.id)}
              <li class="rounded-lg border border-border p-3">
                <div class="flex flex-wrap items-center justify-between gap-2">
                  <span class="font-mono text-xs">{run.id}</span>
                  <Badge variant="outline">{run.state}</Badge>
                </div>
                <p class="mt-2 text-xs text-muted-foreground">
                  {run.kind.replaceAll("_", " ")} · version {run.version}
                </p>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section class="space-y-3" aria-labelledby="recovery-heading">
        <h4 id="recovery-heading" class="font-medium">Local recovery report</h4>
        {#if review.recovery.actions.length === 0}
          <p class="text-sm text-muted-foreground">No recovery actions.</p>
        {:else}
          <ul class="space-y-2 text-sm">
            {#each review.recovery.actions as action (`${action.aggregate_kind}:${action.aggregate_id}:${action.action}`)}
              <li class="rounded-lg border border-border p-3">
                <div class="flex flex-wrap items-center gap-2">
                  <Badge variant="outline">{action.action}</Badge>
                  <span class="font-mono text-xs text-muted-foreground">
                    {action.aggregate_kind}:{action.aggregate_id}
                  </span>
                </div>
                <p class="mt-2">{action.detail}</p>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>

    <section class="space-y-4 rounded-lg border border-border bg-muted/30 p-4" aria-labelledby="finalize-heading">
      <div>
        <p class="text-xs font-medium tracking-wide text-muted-foreground uppercase">
          Irreversible action
        </p>
        <h4 id="finalize-heading" class="mt-1 font-medium">Finalize changeset</h4>
      </div>

      {#if result === null}
        <div class="flex flex-wrap gap-2">
          <Button onclick={previewCommit} disabled={busy}>Preview commit</Button>
          <Button variant="outline" onclick={previewDiscard} disabled={busy}>
            Preview discard
          </Button>
        </div>

        {#if preview !== null}
          <dl class="grid gap-3 text-sm sm:grid-cols-2">
            <div>
              <dt class="text-muted-foreground">Action</dt>
              <dd class="mt-1">{preview.kind}</dd>
            </div>
            <div>
              <dt class="text-muted-foreground">Expected version</dt>
              <dd class="mt-1 font-mono text-xs">{preview.expected_version}</dd>
            </div>
            <div class="sm:col-span-2">
              <dt class="text-muted-foreground">Expected head</dt>
              <dd class="mt-1 break-all font-mono text-xs">{preview.expected_head_sha}</dd>
            </div>
            <div class="sm:col-span-2">
              <dt class="text-muted-foreground">Checkpoint</dt>
              <dd class="mt-1 break-all font-mono text-xs">{preview.checkpoint_id}</dd>
            </div>
            <div class="sm:col-span-2">
              <dt class="text-muted-foreground">Manifest SHA-256</dt>
              <dd class="mt-1 break-all font-mono text-xs">{preview.manifest_sha256}</dd>
            </div>
            <div class="sm:col-span-2">
              <dt class="text-muted-foreground">Confirmation digest</dt>
              <dd class="mt-1 break-all font-mono text-xs">{preview.confirmation_digest}</dd>
            </div>
          </dl>

          <div class="space-y-2">
            <Label for="confirmation-digest">
              Enter the exact confirmation digest to {preview.kind}
            </Label>
            <Input
              id="confirmation-digest"
              bind:value={confirmation}
              autocomplete="off"
              spellcheck="false"
            />
          </div>
          <Button onclick={finalize} disabled={busy || !confirmationMatches}>
            {busy ? "Finalizing…" : `Confirm ${preview.kind}`}
          </Button>
        {/if}
      {:else}
        <div class="space-y-3 text-sm" role="status">
          <p class="font-medium">Changeset {result.kind} completed.</p>
          <p class="break-all font-mono text-xs text-muted-foreground">
            Manifest {result.manifest_sha256}
          </p>
          {#if result.kind === "commit"}
            <p class="break-all font-mono text-xs">
              {result.app_ref} → {result.resulting_head_sha}
            </p>
          {/if}
        </div>
      {/if}
    </section>
  {/if}
</section>
