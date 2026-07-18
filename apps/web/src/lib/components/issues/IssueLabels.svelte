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

<div class="mt-5 border-t border-white/[0.06] pt-4 px-4 pb-0">
  <div class="flex items-center justify-between gap-3">
    <h3 class="text-sm font-medium text-zinc-300">Labels</h3>
    <span class="font-mono text-[10px] text-zinc-700">
      {issue.labels.length} attached
    </span>
  </div>

  <div class="mt-3 flex flex-wrap gap-2">
    {#each issue.labels as label (label._id)}
      <span
        class="rounded-full border px-2 py-0.5 text-[11px]"
        style:background-color={`${label.color}18`}
        style:border-color={`${label.color}44`}
        style:color={label.color}
      >
        {label.name}
      </span>
    {:else}
      <span
        class="rounded-full border border-dashed border-white/10 px-2 py-0.5 text-[11px] text-zinc-600"
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
        class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
        placeholder="New label"
      />
    </label>
    <button
      class="h-9 rounded-md border border-white/10 px-3 text-xs text-zinc-300 transition hover:bg-white/[0.04] disabled:opacity-50"
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
        class="flex items-center gap-2 rounded-md border border-white/[0.06] px-3 py-2 text-sm text-zinc-300 transition hover:bg-white/[0.03]"
      >
        <input
          aria-label={`Toggle ${label.name} label`}
          checked={issueHasLabel(label._id)}
          class="size-4 accent-amber-400"
          onchange={() => onToggleLabel(label._id)}
          type="checkbox"
        />
        <span
          class="size-2 rounded-full"
          style:background-color={label.color}
        ></span>
        <span>{label.name}</span>
      </label>
    {:else}
      <p
        class="rounded-md border border-dashed border-white/10 p-3 text-xs text-zinc-600"
      >
        Create the first label to classify work items.
      </p>
    {/each}
  </div>
</div>
