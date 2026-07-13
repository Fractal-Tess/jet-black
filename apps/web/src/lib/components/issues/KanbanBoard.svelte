<script lang="ts">
import type { Issue, IssueState } from "./types";

let {
  issues,
  onMoveIssue,
  onReorderIssue,
  onSelect,
  selectedIssueId,
  states,
}: {
  issues: Issue[];
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onReorderIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
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
  return issues
    .filter((issue) => issue.stateId === stateId)
    .toSorted((left, right) => {
      const positionDelta = positionForIssue(left) - positionForIssue(right);

      if (positionDelta !== 0) {
        return positionDelta;
      }

      return left.identifier.localeCompare(right.identifier);
    });
}

function positionForIssue(issue: Issue) {
  return issue.position ?? issue._creationTime;
}

function positionBetween(beforeIssue?: Issue, afterIssue?: Issue) {
  if (beforeIssue && afterIssue) {
    return (positionForIssue(beforeIssue) + positionForIssue(afterIssue)) / 2;
  }

  if (beforeIssue) {
    return positionForIssue(beforeIssue) + 1000;
  }

  if (afterIssue) {
    return positionForIssue(afterIssue) - 1000;
  }

  return Date.now();
}

function positionForMoveUp(stateIssues: Issue[], index: number) {
  return positionBetween(stateIssues[index - 2], stateIssues[index - 1]);
}

function positionForMoveDown(stateIssues: Issue[], index: number) {
  return positionBetween(stateIssues[index + 1], stateIssues[index + 2]);
}

function positionAtStateEnd(
  stateId: IssueState["_id"],
  movedIssueId: Issue["_id"]
) {
  const stateIssues = issuesForState(stateId).filter(
    (issue) => issue._id !== movedIssueId
  );

  return positionBetween(stateIssues.at(-1));
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
          {#each stateIssues as issue, index (issue._id)}
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
                <div class="flex items-center gap-1">
                  <button
                    aria-label={`Reorder ${issue.identifier} up`}
                    class="grid size-7 place-items-center rounded-md border border-white/10 text-xs text-zinc-500 transition hover:bg-white/[0.04] hover:text-zinc-200 disabled:opacity-30"
                    disabled={index === 0}
                    onclick={() =>
                      onReorderIssue(
                        issue,
                        state._id,
                        positionForMoveUp(stateIssues, index)
                      )}
                    type="button"
                  >
                    ↑
                  </button>
                  <button
                    aria-label={`Reorder ${issue.identifier} down`}
                    class="grid size-7 place-items-center rounded-md border border-white/10 text-xs text-zinc-500 transition hover:bg-white/[0.04] hover:text-zinc-200 disabled:opacity-30"
                    disabled={index === stateIssues.length - 1}
                    onclick={() =>
                      onReorderIssue(
                        issue,
                        state._id,
                        positionForMoveDown(stateIssues, index)
                      )}
                    type="button"
                  >
                    ↓
                  </button>
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
                      onMoveIssue(
                        issue,
                        stateId,
                        positionAtStateEnd(stateId, issue._id)
                      );
                    }}
                  >
                    {#each states as option (option._id)}
                      <option value={option._id}>{option.name}</option>
                    {/each}
                  </select>
                </div>
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
