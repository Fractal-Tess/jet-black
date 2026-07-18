<script lang="ts">
import type { Issue } from "./types";

let {
  subIssues,
  onCreateSubIssue,
}: {
  subIssues: Issue[];
  onCreateSubIssue: (title: string) => Promise<void>;
} = $props();

let creatingSubIssue = $state(false);
let subIssueTitle = $state("");

async function createSubIssue() {
  const nextTitle = subIssueTitle.trim();

  if (!nextTitle) {
    return;
  }

  creatingSubIssue = true;

  try {
    await onCreateSubIssue(nextTitle);
    subIssueTitle = "";
  } finally {
    creatingSubIssue = false;
  }
}
</script>

<div class="mt-5 border-t border-white/[0.06] pt-4 px-4 pb-0">
  <div class="flex items-center justify-between gap-3">
    <h3 class="text-sm font-medium text-zinc-300">Sub-issues</h3>
    <span class="font-mono text-[10px] text-zinc-700">
      {subIssues.length} child{subIssues.length === 1 ? "" : "ren"}
    </span>
  </div>

  <div class="mt-3 space-y-2">
    {#each subIssues as subIssue (subIssue._id)}
      <article class="rounded-md border border-white/[0.06] p-3">
        <p class="font-mono text-[10px] text-zinc-600">
          {subIssue.identifier}
        </p>
        <p class="mt-1 text-sm text-zinc-300">{subIssue.title}</p>
      </article>
    {:else}
      <p
        class="rounded-md border border-dashed border-white/10 p-3 text-xs text-zinc-600"
      >
        Break this issue into smaller work items.
      </p>
    {/each}
  </div>

  <form
    class="mt-3 flex gap-2"
    onsubmit={(event) => {
      event.preventDefault();
      createSubIssue();
    }}
  >
    <label class="min-w-0 flex-1">
      <span class="sr-only">Sub-issue title</span>
      <input
        bind:value={subIssueTitle}
        class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
        placeholder="Add sub-issue"
      />
    </label>
    <button
      class="h-9 rounded-md border border-white/10 px-3 text-xs text-zinc-300 transition hover:bg-white/[0.04] disabled:opacity-50"
      disabled={creatingSubIssue || !subIssueTitle.trim()}
      type="submit"
    >
      {creatingSubIssue ? "Adding\u2026" : "Add"}
    </button>
  </form>
</div>
