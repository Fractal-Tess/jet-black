<script lang="ts">
import type { Issue, IssueComment, IssuePriority, IssueState } from "./types";

let {
  comments,
  issue,
  onAddComment,
  onUpdateIssue,
  states,
}: {
  comments: IssueComment[];
  issue: Issue | null;
  onAddComment: (body: string) => Promise<void>;
  onUpdateIssue: (input: {
    description?: string;
    priority?: IssuePriority;
    stateId?: IssueState["_id"];
    title?: string;
  }) => Promise<void>;
  states: IssueState[];
} = $props();

let editing = $state(false);
let saving = $state(false);
let commentBody = $state("");
let title = $state("");
let description = $state("");
let priority = $state<IssuePriority>("none");
let stateId = $state<IssueState["_id"] | undefined>();

$effect(() => {
  if (!issue) {
    return;
  }

  title = issue.title;
  description = issue.description ?? "";
  priority = issue.priority;
  stateId = issue.stateId;
});

async function saveIssue() {
  if (!issue) {
    return;
  }

  saving = true;

  try {
    await onUpdateIssue({
      description,
      priority,
      stateId,
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

        <div>
          <p class="mb-2 text-xs text-zinc-600">Description</p>
          <p class="rounded-md border border-white/[0.06] p-3 text-sm text-zinc-400">
            {issue.description || "No description yet."}
          </p>
        </div>
      {/if}

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
