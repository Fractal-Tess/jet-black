<script lang="ts">
import Plus from "lucide-svelte/icons/plus";
import X from "lucide-svelte/icons/x";
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

<div class="border-b border-white/[0.06] px-5 py-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-medium text-zinc-300">Sub-issues</h3>
    <span class="text-xs text-zinc-600">{subIssues.length} children</span>
  </div>

  {#if subIssues.length > 0}
    <div class="mt-3 space-y-1.5">
      {#each subIssues as sub (sub._id)}
        <div
          class="flex items-center gap-2 rounded-md px-2 py-1.5 transition hover:bg-white/[0.03]"
        >
          <span
            class="size-2 rounded-full"
            style:background-color={sub.state?.color ?? "#71717a"}
          ></span>
          <span class="font-mono text-[11px] text-zinc-500"
            >{sub.identifier}</span
          >
          <span class="text-sm text-zinc-300">{sub.title}</span>
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
        class="h-8 min-w-0 flex-1 rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-amber-400/60"
        placeholder="Sub-issue title"
      />
      <button
        class="h-8 rounded-md bg-amber-400 px-3 text-xs font-medium text-black transition hover:bg-amber-300 disabled:opacity-50"
        disabled={creatingSub || !subIssueTitle.trim()}
        type="submit"
      >
        {creatingSub ? "Adding\u2026" : "Add"}
      </button>
      <button
        class="grid size-8 place-items-center rounded-md border border-white/10 text-zinc-500 transition hover:bg-white/[0.04] hover:text-zinc-300"
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
      class="mt-3 flex items-center gap-1.5 text-sm text-zinc-500 transition hover:text-zinc-300"
      onclick={() => (showSubIssueForm = true)}
      type="button"
    >
      <Plus class="size-3.5" />
      Add sub-issue
    </button>
  {/if}
</div>
