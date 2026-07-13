<script lang="ts">
import type { Issue, IssuePriority, IssueState } from "./types";

type SortKey = "identifier" | "priority" | "state" | "title" | "updated";

let {
  issues,
  onSelect,
  selectedIssueId,
  states,
}: {
  issues: Issue[];
  onSelect: (issue: Issue) => void;
  selectedIssueId?: string;
  states: IssueState[];
} = $props();

let priorityFilter = $state<IssuePriority | "all">("all");
let sortDirection = $state<"asc" | "desc">("asc");
let sortKey = $state<SortKey>("identifier");
let stateFilter = $state<IssueState["_id"] | "all">("all");

const priorityWeight: Record<IssuePriority, number> = {
  none: 0,
  low: 1,
  medium: 2,
  high: 3,
  urgent: 4,
};

const filteredIssues = $derived(
  issues
    .filter((issue) => stateFilter === "all" || issue.stateId === stateFilter)
    .filter(
      (issue) => priorityFilter === "all" || issue.priority === priorityFilter
    )
    .toSorted((left, right) => compareIssues(left, right))
);

function compareIssues(left: Issue, right: Issue) {
  const direction = sortDirection === "asc" ? 1 : -1;

  if (sortKey === "priority") {
    return (
      (priorityWeight[left.priority] - priorityWeight[right.priority]) *
      direction
    );
  }

  if (sortKey === "state") {
    return stateName(left).localeCompare(stateName(right)) * direction;
  }

  if (sortKey === "updated") {
    return (left.updatedAt - right.updatedAt) * direction;
  }

  return (
    String(left[sortKey]).localeCompare(String(right[sortKey])) * direction
  );
}

function stateName(issue: Issue) {
  return issue.state?.name ?? "No state";
}

function setSort(nextSortKey: SortKey) {
  if (sortKey === nextSortKey) {
    sortDirection = sortDirection === "asc" ? "desc" : "asc";
    return;
  }

  sortKey = nextSortKey;
  sortDirection = "asc";
}

function sortLabel(label: string, key: SortKey) {
  if (sortKey !== key) {
    return label;
  }

  return `${label} ${sortDirection === "asc" ? "↑" : "↓"}`;
}
</script>

<section class="rounded-xl border border-white/10 bg-[#151616]">
  <div
    class="flex flex-col gap-3 border-b border-white/[0.06] px-4 py-3 lg:flex-row lg:items-center lg:justify-between"
  >
    <div>
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-zinc-500">
        List
      </p>
      <h2 class="mt-1 text-lg font-semibold text-zinc-100">Issues</h2>
    </div>

    <div class="flex flex-wrap gap-2">
      <label>
        <span class="sr-only">Filter by state</span>
        <select
          bind:value={stateFilter}
          class="h-8 rounded-md border border-white/10 bg-[#101111] px-2 text-xs text-zinc-300 outline-none focus:border-amber-400/60"
        >
          <option value="all">All states</option>
          {#each states as state (state._id)}
            <option value={state._id}>{state.name}</option>
          {/each}
        </select>
      </label>

      <label>
        <span class="sr-only">Filter by priority</span>
        <select
          bind:value={priorityFilter}
          class="h-8 rounded-md border border-white/10 bg-[#101111] px-2 text-xs text-zinc-300 outline-none focus:border-amber-400/60"
        >
          <option value="all">All priorities</option>
          <option value="none">None</option>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
          <option value="urgent">Urgent</option>
        </select>
      </label>
    </div>
  </div>

  <div class="overflow-x-auto">
    <table class="w-full min-w-[760px] text-left text-sm">
      <thead class="border-b border-white/[0.06] text-xs text-zinc-600">
        <tr>
          <th class="px-4 py-2 font-medium">
            <button type="button" onclick={() => setSort("identifier")}>
              {sortLabel("ID", "identifier")}
            </button>
          </th>
          <th class="px-4 py-2 font-medium">
            <button type="button" onclick={() => setSort("title")}>
              {sortLabel("Title", "title")}
            </button>
          </th>
          <th class="px-4 py-2 font-medium">
            <button type="button" onclick={() => setSort("state")}>
              {sortLabel("State", "state")}
            </button>
          </th>
          <th class="px-4 py-2 font-medium">
            <button type="button" onclick={() => setSort("priority")}>
              {sortLabel("Priority", "priority")}
            </button>
          </th>
          <th class="px-4 py-2 font-medium">
            <button type="button" onclick={() => setSort("updated")}>
              {sortLabel("Updated", "updated")}
            </button>
          </th>
        </tr>
      </thead>
      <tbody>
        {#each filteredIssues as issue (issue._id)}
          <tr
            class="border-b border-white/[0.04] transition hover:bg-white/[0.03] {selectedIssueId ===
            issue._id
              ? 'bg-amber-400/[0.04]'
              : ''}"
          >
            <td class="px-4 py-3 font-mono text-xs text-zinc-500">
              {issue.identifier}
            </td>
            <td class="px-4 py-3">
              <button
                class="text-left font-medium text-zinc-100 hover:text-amber-300"
                onclick={() => onSelect(issue)}
                type="button"
              >
                {issue.title}
              </button>
            </td>
            <td class="px-4 py-3">
              <span class="inline-flex items-center gap-2 text-xs text-zinc-400">
                <span
                  class="size-2 rounded-full"
                  style:background-color={issue.state?.color ?? "#71717a"}
                ></span>
                {stateName(issue)}
              </span>
            </td>
            <td class="px-4 py-3 capitalize text-xs text-zinc-400">
              {issue.priority}
            </td>
            <td class="px-4 py-3 font-mono text-xs text-zinc-600">
              {new Date(issue.updatedAt).toLocaleDateString()}
            </td>
          </tr>
        {:else}
          <tr>
            <td class="px-4 py-10 text-center text-xs text-zinc-600" colspan="5">
              No issues match these filters.
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>
