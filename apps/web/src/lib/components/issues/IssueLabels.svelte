<script lang="ts">
import type { CreateLabelInput, Issue, IssueLabel } from "./types";

let {
  issue,
  labels,
  onCreateLabel,
  onToggleLabel,
}: {
  issue: Issue;
  labels: IssueLabel[];
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
} = $props();

let creatingLabel = $state(false);
let newLabelName = $state("");

function issueHasLabel(labelId: IssueLabel["_id"]) {
  return issue?.labels.some((label) => label._id === labelId) ?? false;
}

async function createLabel() {
  const name = newLabelName.trim();

  if (!name) {
    return;
  }

  creatingLabel = true;

  try {
    await onCreateLabel({ name });
    newLabelName = "";
  } finally {
    creatingLabel = false;
  }
}
</script>

<div class="mt-5 border-t border-border pt-4 px-4 pb-0">
  <div class="flex items-center justify-between gap-3">
    <h3 class="text-sm font-medium text-foreground">Labels</h3>
    <span class="font-mono text-meta text-muted-foreground">
      {issue.labels.length} attached
    </span>
  </div>

  <div class="mt-3 flex flex-wrap gap-2">
    {#each issue.labels as label (label._id)}
      <span class="rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 text-meta text-primary">
        {label.name}
      </span>
    {:else}
      <span
        class="rounded-full border border-dashed border-border px-2 py-0.5 text-meta text-muted-foreground"
      >
        No labels
      </span>
    {/each}
  </div>

  <div class="mt-3 flex gap-2">
    <label class="min-w-0 flex-1">
      <span class="sr-only">New label name</span>
      <input
        bind:value={newLabelName}
        class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="New label"
      />
    </label>
    <button
      class="h-9 rounded-lg border border-border px-3 text-xs text-foreground transition-colors hover:bg-muted disabled:opacity-50"
      disabled={creatingLabel || !newLabelName.trim()}
      onclick={createLabel}
      type="button"
    >
      {creatingLabel ? "Creating\u2026" : "Create label"}
    </button>
  </div>

  <div class="mt-3 grid gap-2">
    {#each labels as label (label._id)}
      <label
        class="flex items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm text-foreground transition-colors hover:bg-muted"
      >
        <input
          aria-label={`Toggle ${label.name} label`}
          checked={issueHasLabel(label._id)}
          class="size-4 accent-primary"
          onchange={() => onToggleLabel(label._id)}
          type="checkbox"
        />
        <span class="size-2 rounded-full bg-primary"></span>
        <span>{label.name}</span>
      </label>
    {:else}
      <p
        class="rounded-lg border border-dashed border-border p-3 text-xs text-muted-foreground"
      >
        Create the first label to classify work items.
      </p>
    {/each}
  </div>
</div>
