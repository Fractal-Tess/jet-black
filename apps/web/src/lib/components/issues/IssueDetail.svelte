<script lang="ts">
import type {
  Issue,
  IssueComment,
  IssueLabel,
  IssuePriority,
  IssueState,
} from "./types";

let {
  comments,
  issue,
  labels,
  onAddComment,
  onCreateLabel,
  onToggleLabel,
  onUpdateIssue,
  states,
}: {
  comments: IssueComment[];
  issue: Issue | null;
  labels: IssueLabel[];
  onAddComment: (body: string) => Promise<void>;
  onCreateLabel: (input: { color?: string; name: string }) => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: {
    description?: string;
    estimate?: number | null;
    priority?: IssuePriority;
    stateId?: IssueState["_id"];
    startDate?: string | null;
    targetDate?: string | null;
    title?: string;
  }) => Promise<void>;
  states: IssueState[];
} = $props();

let editing = $state(false);
let saving = $state(false);
let commentBody = $state("");
let creatingLabel = $state(false);
let newLabelName = $state("");
let title = $state("");
let description = $state("");
let estimate = $state("");
let priority = $state<IssuePriority>("none");
let startDate = $state("");
let stateId = $state<IssueState["_id"] | undefined>();
let targetDate = $state("");

$effect(() => {
  if (!issue) {
    return;
  }

  title = issue.title;
  description = issue.description ?? "";
  estimate = issue.estimate?.toString() ?? "";
  priority = issue.priority;
  startDate = issue.startDate ?? "";
  stateId = issue.stateId;
  targetDate = issue.targetDate ?? "";
});

async function saveIssue() {
  if (!issue) {
    return;
  }

  const normalizedEstimate = String(estimate).trim();

  saving = true;

  try {
    await onUpdateIssue({
      description,
      estimate: normalizedEstimate ? Number(normalizedEstimate) : null,
      priority,
      stateId,
      startDate: startDate || null,
      targetDate: targetDate || null,
      title,
    });
    editing = false;
  } finally {
    saving = false;
  }
}

async function addComment() {
  const body = commentBody.trim();

  if (!body) {
    return;
  }

  await onAddComment(body);
  commentBody = "";
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

function issueHasLabel(labelId: IssueLabel["_id"]) {
  return issue?.labels.some((label) => label._id === labelId) ?? false;
}
</script>

<aside class="rounded-xl border border-white/10 bg-[#151616]">
  {#if issue}
    <div class="border-b border-white/[0.06] px-4 py-3">
      <div class="flex items-center justify-between gap-3">
        <div>
          <p class="font-mono text-[11px] text-zinc-500">{issue.identifier}</p>
          <h2 class="mt-1 text-lg font-semibold text-zinc-100">
            {editing ? "Edit issue" : issue.title}
          </h2>
        </div>
        <button
          class="h-8 rounded-md border border-white/10 px-3 text-xs text-zinc-400 transition hover:bg-white/[0.04] hover:text-zinc-100"
          onclick={() => (editing = !editing)}
          type="button"
        >
          {editing ? "Cancel" : "Edit"}
        </button>
      </div>
    </div>

    <div class="space-y-5 p-4">
      {#if editing}
        <div class="space-y-3">
          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">Title</span>
            <input
              bind:value={title}
              class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none focus:border-amber-400/60"
            />
          </label>

          <label class="block">
            <span class="mb-1 block text-xs text-zinc-500">Description</span>
            <textarea
              bind:value={description}
              class="min-h-24 w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none focus:border-amber-400/60"
            ></textarea>
          </label>

          <div class="grid gap-3 sm:grid-cols-2">
            <label class="block">
              <span class="mb-1 block text-xs text-zinc-500">State</span>
              <select
                bind:value={stateId}
                class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-200 outline-none focus:border-amber-400/60"
                data-testid="issue-state-select"
              >
                {#each states as state (state._id)}
                  <option value={state._id}>{state.name}</option>
                {/each}
              </select>
            </label>

            <label class="block">
              <span class="mb-1 block text-xs text-zinc-500">Priority</span>
              <select
                bind:value={priority}
                class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-200 outline-none focus:border-amber-400/60"
              >
                <option value="none">None</option>
                <option value="low">Low</option>
                <option value="medium">Medium</option>
                <option value="high">High</option>
                <option value="urgent">Urgent</option>
              </select>
            </label>
          </div>

          <div class="grid gap-3 sm:grid-cols-3">
            <label class="block">
              <span class="mb-1 block text-xs text-zinc-500">Estimate</span>
              <input
                bind:value={estimate}
                class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-200 outline-none focus:border-amber-400/60"
                min="0"
                placeholder="Points"
                type="number"
              />
            </label>

            <label class="block">
              <span class="mb-1 block text-xs text-zinc-500">Start date</span>
              <input
                bind:value={startDate}
                class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-200 outline-none focus:border-amber-400/60"
                type="date"
              />
            </label>

            <label class="block">
              <span class="mb-1 block text-xs text-zinc-500">Target date</span>
              <input
                bind:value={targetDate}
                class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-200 outline-none focus:border-amber-400/60"
                type="date"
              />
            </label>
          </div>

          <button
            class="h-9 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
            disabled={saving || !title.trim()}
            onclick={saveIssue}
            type="button"
          >
            {saving ? "Saving…" : "Save changes"}
          </button>
        </div>
      {:else}
        <div class="grid gap-3 text-sm sm:grid-cols-2">
          <div class="rounded-md border border-white/[0.06] p-3">
            <p class="text-xs text-zinc-600">State</p>
            <p class="mt-1 flex items-center gap-2 text-zinc-300">
              <span
                class="size-2 rounded-full"
                style:background-color={issue.state?.color ?? "#71717a"}
              ></span>
              {issue.state?.name ?? "No state"}
            </p>
          </div>
          <div class="rounded-md border border-white/[0.06] p-3">
            <p class="text-xs text-zinc-600">Priority</p>
            <p class="mt-1 capitalize text-zinc-300">{issue.priority}</p>
          </div>
        </div>

        <div class="grid gap-3 text-sm sm:grid-cols-3">
          <div class="rounded-md border border-white/[0.06] p-3">
            <p class="text-xs text-zinc-600">Estimate</p>
            <p class="mt-1 text-zinc-300">
              {issue.estimate !== undefined ? `${issue.estimate} points` : "No estimate"}
            </p>
          </div>
          <div class="rounded-md border border-white/[0.06] p-3">
            <p class="text-xs text-zinc-600">Start date</p>
            <p class="mt-1 text-zinc-300">{issue.startDate ?? "Not set"}</p>
          </div>
          <div class="rounded-md border border-white/[0.06] p-3">
            <p class="text-xs text-zinc-600">Target date</p>
            <p class="mt-1 text-zinc-300">{issue.targetDate ?? "Not set"}</p>
          </div>
        </div>

        <div>
          <p class="mb-2 text-xs text-zinc-600">Description</p>
          <p class="rounded-md border border-white/[0.06] p-3 text-sm text-zinc-400">
            {issue.description || "No description yet."}
          </p>
        </div>
      {/if}

      <div class="border-t border-white/[0.06] pt-4">
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
            {creatingLabel ? "Creating…" : "Create label"}
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

      <div class="border-t border-white/[0.06] pt-4">
        <h3 class="text-sm font-medium text-zinc-300">Comments</h3>
        <div class="mt-3 space-y-3">
          {#each comments as comment (comment._id)}
            <article class="rounded-md border border-white/[0.06] p-3">
              <p class="text-sm text-zinc-300">{comment.body}</p>
              <p class="mt-2 font-mono text-[10px] text-zinc-700">
                {comment.authorUserId.slice(0, 8)}
              </p>
            </article>
          {:else}
            <p class="rounded-md border border-dashed border-white/10 p-4 text-sm text-zinc-600">
              No comments yet.
            </p>
          {/each}
        </div>

        <label class="mt-3 block">
          <span class="sr-only">New comment</span>
          <textarea
            bind:value={commentBody}
            class="min-h-20 w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
            placeholder="Add a comment…"
          ></textarea>
        </label>
        <button
          class="mt-2 h-8 rounded-md border border-white/10 px-3 text-xs text-zinc-300 transition hover:bg-white/[0.04] disabled:opacity-50"
          disabled={!commentBody.trim()}
          onclick={addComment}
          type="button"
        >
          Comment
        </button>
      </div>
    </div>
  {:else}
    <div class="grid min-h-80 place-items-center p-8 text-center">
      <div>
        <div
          class="mx-auto grid size-12 place-items-center rounded-xl border border-white/[0.06] bg-white/[0.02] text-xl text-zinc-700"
        >
          ↗
        </div>
        <p class="mt-4 text-sm text-zinc-500">Select an issue to inspect it.</p>
      </div>
    </div>
  {/if}
</aside>
