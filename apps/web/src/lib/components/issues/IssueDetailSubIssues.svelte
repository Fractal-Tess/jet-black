<script lang="ts">
import Plus from "lucide-svelte/icons/plus";
import X from "lucide-svelte/icons/x";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type { Issue } from "./types";

let {
  subIssues,
  showSubIssueForm = $bindable(),
  onCreateSubIssue,
}: {
  subIssues: Issue[];
  showSubIssueForm: boolean;
  onCreateSubIssue: (title: string) => Promise<void>;
} = $props();

let subIssueTitle = $state("");
let creatingSub = $state(false);

async function createSubIssue() {
  const title = subIssueTitle.trim();
  if (!title) {
    return;
  }
  creatingSub = true;
  try {
    await onCreateSubIssue(title);
    subIssueTitle = "";
    showSubIssueForm = false;
  } finally {
    creatingSub = false;
  }
}
</script>

<div class="border-b border-border px-5 py-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-medium text-foreground">Sub-issues</h3>
    <span class="text-xs text-muted-foreground">{subIssues.length} children</span>
  </div>

  {#if subIssues.length > 0}
    <div class="mt-3 space-y-1.5">
      {#each subIssues as sub (sub._id)}
        <div
          class="flex items-center gap-2 rounded-lg px-2 py-1.5 transition-colors hover:bg-muted"
        >
          {#if sub.state}<StateTypeIcon class="size-3.5" type={sub.state.type} />{/if}
          <span class="font-mono text-meta text-muted-foreground"
            >{sub.identifier}</span
          >
          <span class="text-sm text-foreground">{sub.title}</span>
        </div>
      {/each}
    </div>
  {/if}

  {#if showSubIssueForm}
    <form
      class="mt-3 flex gap-2"
      onsubmit={(e) => {
        e.preventDefault();
        createSubIssue();
      }}
    >
      <input
        bind:value={subIssueTitle}
        class="h-8 min-w-0 flex-1 rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="Sub-issue title"
      />
      <button
        class="h-8 rounded-lg bg-primary px-3 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/80 disabled:opacity-50"
        disabled={creatingSub || !subIssueTitle.trim()}
        type="submit"
      >
        {creatingSub ? "Adding\u2026" : "Add"}
      </button>
      <button
        class="grid size-8 place-items-center rounded-lg border border-border text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        onclick={() => {
          showSubIssueForm = false;
          subIssueTitle = "";
        }}
        type="button"
      >
        <X class="size-3.5" />
      </button>
    </form>
  {:else if subIssues.length === 0}
    <button
      class="mt-3 flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
      onclick={() => (showSubIssueForm = true)}
      type="button"
    >
      <Plus class="size-3.5" />
      Add sub-issue
    </button>
  {/if}
</div>
