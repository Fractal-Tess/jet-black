<script lang="ts">
import type { Issue, IssueState } from "./types";

let {
  issues,
  onMoveIssue,
  onSelect,
  selectedIssueId,
  states,
}: {
  issues: Issue[];
  onMoveIssue: (issue: Issue, stateId: IssueState["_id"]) => Promise<void>;
  onSelect: (issue: Issue) => void;
  selectedIssueId?: string;
  states: IssueState[];
} = $props();

const priorityLabel: Record<Issue["priority"], string> = {
  high: "High",
  low: "Low",
  medium: "Medium",
  none: "None",
  urgent: "Urgent",
};

function issuesForState(stateId: IssueState["_id"]) {
  return issues.filter((issue) => issue.stateId === stateId);
}
</script>

<section class="rounded-xl border border-white/10 bg-[#151616]">
  <div
    class="flex items-center justify-between border-b border-white/[0.06] px-4 py-3"
  >
    <div>
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
        Board
      </p>
      <h2 class="mt-1 text-lg font-semibold text-zinc-100">Kanban</h2>
    </div>
    <span class="rounded-md border border-white/10 px-2 py-1 text-xs text-zinc-500">
      {issues.length} total
    </span>
  </div>

  <div class="grid gap-3 overflow-x-auto p-3 lg:grid-cols-4">
    {#each states as state (state._id)}
      {@const stateIssues = issuesForState(state._id)}
      <section
        aria-label={`${state.name} column`}
        class="min-h-80 min-w-64 rounded-lg border border-white/[0.06] bg-[#101111]"
      >
        <div class="flex items-center justify-between border-b border-white/[0.06] px-3 py-2">
          <div class="flex min-w-0 items-center gap-2">
            <span
              class="size-2 rounded-full"
              style:background-color={state.color}
            ></span>
            <h3 class="truncate text-sm font-medium text-zinc-200">
              {state.name}
            </h3>
          </div>
          <span class="font-mono text-xs text-zinc-600">
            {stateIssues.length}
          </span>
        </div>

        <div class="space-y-2 p-2">
          {#each stateIssues as issue (issue._id)}
            <article
              class="rounded-lg border border-white/[0.06] bg-white/[0.025] p-3 shadow-lg shadow-black/10 transition hover:-translate-y-0.5 hover:border-white/15 hover:bg-white/[0.04] {selectedIssueId ===
              issue._id
                ? 'border-amber-400/40 bg-amber-400/[0.04]'
                : ''}"
            >
              <button
                class="block w-full text-left"
                onclick={() => onSelect(issue)}
                type="button"
              >
                <span class="font-mono text-[11px] text-zinc-500">
                  {issue.identifier}
                </span>
                <span class="mt-1 block text-sm font-medium leading-5 text-zinc-100">
                  {issue.title}
                </span>
                {#if issue.description}
                  <span class="mt-1 line-clamp-2 block text-xs leading-5 text-zinc-600">
                    {issue.description}
                  </span>
                {/if}
                {#if issue.labels.length > 0}
                  <span class="mt-3 flex flex-wrap gap-1">
                    {#each issue.labels as label (label._id)}
                      <span
                        class="rounded-full border px-1.5 py-0.5 text-[10px]"
                        style:background-color={`${label.color}18`}
                        style:border-color={`${label.color}44`}
                        style:color={label.color}
                      >
                        {label.name}
                      </span>
                    {/each}
                  </span>
                {/if}
              </button>

              <div class="mt-3 flex items-center justify-between gap-2">
                <span
                  class="rounded-full border border-amber-400/15 bg-amber-400/5 px-2 py-0.5 text-[11px] text-amber-300"
                >
                  {priorityLabel[issue.priority]}
                </span>
                <label class="sr-only" for={`move-${issue._id}`}>
                  Move {issue.identifier}
                </label>
                <select
                  class="h-7 max-w-28 rounded-md border border-white/10 bg-[#0f1010] px-1 text-[11px] text-zinc-400 outline-none focus:border-amber-400/60"
                  id={`move-${issue._id}`}
                  value={issue.stateId}
                  onchange={(event) => {
                    const stateId = event.currentTarget
                      .value as IssueState["_id"];
                    onMoveIssue(issue, stateId);
                  }}
                >
                  {#each states as option (option._id)}
                    <option value={option._id}>{option.name}</option>
                  {/each}
                </select>
              </div>
            </article>
          {:else}
            <div
              class="rounded-lg border border-dashed border-white/[0.06] px-3 py-8 text-center text-xs text-zinc-700"
            >
              No work items
            </div>
          {/each}
        </div>
      </section>
    {/each}
  </div>
</section>
