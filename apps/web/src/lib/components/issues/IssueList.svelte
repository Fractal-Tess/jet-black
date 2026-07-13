<script lang="ts">
import type { Issue } from "./types";

let {
  issues,
  selectedIssueId,
  onSelect,
}: {
  issues: Issue[];
  onSelect: (issue: Issue) => void;
  selectedIssueId?: string;
} = $props();

const priorityLabel: Record<Issue["priority"], string> = {
  high: "High",
  low: "Low",
  medium: "Medium",
  none: "None",
  urgent: "Urgent",
};

function formatAge(timestamp: number) {
  const days = Math.max(0, Math.floor((Date.now() - timestamp) / 86_400_000));

  if (days === 0) {
    return "today";
  }

  if (days === 1) {
    return "1 day ago";
  }

  return `${days} days ago`;
}
</script>

<section class="rounded-xl border border-white/10 bg-[#151616]">
  <div
    class="flex items-center justify-between border-b border-white/[0.06] px-4 py-3"
  >
    <div>
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
        Tickets
      </p>
      <h2 class="mt-1 text-lg font-semibold text-zinc-100">
        Work item inbox
      </h2>
    </div>
    <span class="rounded-md border border-white/10 px-2 py-1 text-xs text-zinc-500">
      {issues.length} total
    </span>
  </div>

  <div class="divide-y divide-white/[0.05]">
    {#each issues as issue (issue._id)}
      <button
        class="grid w-full grid-cols-[auto_1fr] gap-3 px-4 py-3 text-left transition hover:bg-white/[0.03] sm:grid-cols-[auto_1fr_auto] {selectedIssueId ===
        issue._id
          ? 'bg-white/[0.05]'
          : ''}"
        onclick={() => onSelect(issue)}
        type="button"
      >
        <span
          class="mt-0.5 grid size-8 place-items-center rounded-md bg-white/[0.04] font-mono text-[10px] text-zinc-500"
        >
          {issue.identifier.split("-").at(-1)}
        </span>

        <span class="min-w-0">
          <span class="flex flex-wrap items-center gap-2">
            <span class="font-mono text-[11px] text-zinc-500">
              {issue.identifier}
            </span>
            <span
              class="inline-flex items-center gap-1 rounded-full border border-white/10 px-2 py-0.5 text-[11px] text-zinc-400"
            >
              <span
                class="size-1.5 rounded-full"
                style:background-color={issue.state?.color ?? "#71717a"}
              ></span>
              {issue.state?.name ?? "No state"}
            </span>
            {#if issue.priority !== "none"}
              <span
                class="rounded-full border border-amber-400/20 bg-amber-400/5 px-2 py-0.5 text-[11px] text-amber-300"
              >
                {priorityLabel[issue.priority]}
              </span>
            {/if}
          </span>
          <span class="mt-1 block truncate text-sm text-zinc-200">
            {issue.title}
          </span>
          {#if issue.description}
            <span class="mt-1 block truncate text-xs text-zinc-600">
              {issue.description}
            </span>
          {/if}
        </span>

        <span class="hidden self-center text-xs text-zinc-600 sm:block">
          {formatAge(issue.updatedAt)}
        </span>
      </button>
    {:else}
      <div class="px-4 py-12 text-center">
        <div
          class="mx-auto grid size-12 place-items-center rounded-xl border border-white/[0.06] bg-white/[0.02] text-xl text-zinc-700"
        >
          ◇
        </div>
        <p class="mt-4 text-sm text-zinc-500">No tickets yet.</p>
        <p class="mt-1 text-xs text-zinc-700">
          Create the first work item to start the project loop.
        </p>
      </div>
    {/each}
  </div>
</section>
