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

<div class="mt-5 border-t border-border pt-4 px-4 pb-0">
  <div class="flex items-center justify-between gap-3">
    <h3 class="text-sm font-medium text-foreground">Sub-issues</h3>
    <span class="font-mono text-meta text-muted-foreground">
      {subIssues.length} child{subIssues.length === 1 ? "" : "ren"}
    </span>
  </div>

  <div class="mt-3 space-y-2">
    {#each subIssues as subIssue (subIssue._id)}
      <article class="rounded-lg border border-border p-3">
        <p class="font-mono text-meta text-muted-foreground">
          {subIssue.identifier}
        </p>
        <p class="mt-1 text-sm text-foreground">{subIssue.title}</p>
      </article>
    {:else}
      <p
      class="rounded-lg border border-dashed border-border p-3 text-xs text-muted-foreground"
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
        class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="Add sub-issue"
      />
    </label>
    <button
      class="h-9 rounded-lg border border-border px-3 text-xs text-foreground transition-colors hover:bg-muted disabled:opacity-50"
      disabled={creatingSubIssue || !subIssueTitle.trim()}
      type="submit"
    >
      {creatingSubIssue ? "Adding\u2026" : "Add"}
    </button>
  </form>
</div>
