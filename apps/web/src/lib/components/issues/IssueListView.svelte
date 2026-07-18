<script lang="ts">
import ChevronRight from "lucide-svelte/icons/chevron-right";
import Plus from "lucide-svelte/icons/plus";
import {
  compareIssues,
  groupIssues,
  type IssueDisplayOptions,
  type IssueGroup,
} from "./display-options";
import IssuePropertyChips from "./IssuePropertyChips.svelte";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type {
  Issue,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  displayOptions,
  issues,
  members,
  onMoveIssue,
  onQuickCreate,
  onSelect,
  onUpdateIssueFor,
  selectedIssueId,
  states,
}: {
  displayOptions: IssueDisplayOptions;
  issues: Issue[];
  members: WorkspaceMember[];
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onQuickCreate: (state: IssueState, title: string) => Promise<void>;
  onSelect: (issue: Issue) => void;
  onUpdateIssueFor: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  selectedIssueId?: string;
  states: IssueState[];
} = $props();

let collapsedGroupIds = $state<string[]>([]);
let creatingGroupId = $state<string | null>(null);
let quickTitles = $state<Record<string, string>>({});

const groups = $derived(groupIssues(issues, states, displayOptions));

function isCollapsed(groupId: string) {
  return collapsedGroupIds.includes(groupId);
}

function toggleGroup(groupId: string) {
  collapsedGroupIds = isCollapsed(groupId)
    ? collapsedGroupIds.filter((id) => id !== groupId)
    : [...collapsedGroupIds, groupId];
}

function positionForIssue(issue: Issue) {
  return issue.position ?? issue._creationTime;
}

async function handleMoveToState(issue: Issue, stateId: IssueState["_id"]) {
  const targetIssues = issues
    .filter(
      (candidate) =>
        candidate.stateId === stateId && candidate._id !== issue._id
    )
    .toSorted((left, right) => compareIssues(left, right, "manual"));
  const lastIssue = targetIssues.at(-1);
  const position = lastIssue ? positionForIssue(lastIssue) + 1000 : Date.now();

  await onMoveIssue(issue, stateId, position);
}

async function submitQuickCreate(group: IssueGroup) {
  const title = (quickTitles[group.id] ?? "").trim();

  if (!(title && group.state)) {
    return;
  }

  creatingGroupId = group.id;

  try {
    await onQuickCreate(group.state, title);
    quickTitles = { ...quickTitles, [group.id]: "" };
  } finally {
    creatingGroupId = null;
  }
}
</script>

<div class="flex flex-col">
  {#each groups as group (group.id)}
    <section aria-label={`${group.name} group`} class="border-b border-white/[0.06]">
      <div class="flex items-center gap-2 px-3 py-2.5">
        <button
          aria-expanded={!isCollapsed(group.id)}
          aria-label={`Toggle ${group.name} group`}
          class="grid size-5 place-items-center rounded text-zinc-600 transition hover:bg-white/[0.06] hover:text-zinc-300"
          onclick={() => toggleGroup(group.id)}
          type="button"
        >
          <ChevronRight
            class="size-3.5 transition-transform {isCollapsed(group.id)
              ? ''
              : 'rotate-90'}"
          />
        </button>
        {#if group.state}
          <StateTypeIcon
            class="size-3.5"
            color={group.state.color}
            type={group.state.type}
          />
        {:else if group.priority}
          <PriorityIcon class="size-3.5" priority={group.priority} />
        {/if}
        <h3 class="text-sm font-medium text-zinc-200">{group.name}</h3>
        <span class="text-sm tabular-nums text-zinc-500">
          {group.issues.length}
        </span>
        {#if group.state}
          <button
            aria-label={`New work item in ${group.name}`}
            class="ml-1 grid size-5 place-items-center rounded text-zinc-600 transition hover:bg-white/[0.06] hover:text-zinc-300"
            onclick={() =>
              document.getElementById(`list-quick-add-${group.id}`)?.focus()}
            type="button"
          >
            <Plus class="size-3.5" />
          </button>
        {/if}
      </div>

      {#if !isCollapsed(group.id)}
        <div>
          {#each group.issues as issue (issue._id)}
            <div
              class="flex min-h-11 items-center gap-3 border-t border-white/[0.04] py-2 pr-3 pl-10 transition hover:bg-white/[0.03] {selectedIssueId ===
              issue._id
                ? 'bg-amber-400/[0.04]'
                : ''}"
            >
              {#if displayOptions.properties.key}
                <span
                  class="w-16 shrink-0 font-mono text-[11px] text-zinc-500"
                >
                  {issue.identifier}
                </span>
              {/if}
              <button
                class="min-w-0 flex-1 cursor-pointer truncate text-left text-[13px] font-medium text-zinc-100 transition hover:text-amber-300"
                onclick={() => onSelect(issue)}
                type="button"
              >
                {issue.title}
              </button>
              <div class="hidden shrink-0 sm:block">
                <IssuePropertyChips
                  {issue}
                  {members}
                  onMoveToState={(stateId) => handleMoveToState(issue, stateId)}
                  onUpdate={(input) => onUpdateIssueFor(issue, input)}
                  properties={displayOptions.properties}
                  {states}
                />
              </div>
            </div>
          {/each}

          {#if group.state}
            <form
              class="flex items-center gap-2 border-t border-white/[0.04] py-2 pr-3 pl-10 transition focus-within:bg-white/[0.02]"
              onsubmit={(event) => {
                event.preventDefault();
                submitQuickCreate(group);
              }}
            >
              <Plus class="size-3.5 shrink-0 text-zinc-600" />
              <input
                aria-label={`Quick issue title for ${group.name}`}
                class="min-w-0 flex-1 bg-transparent text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600"
                id={`list-quick-add-${group.id}`}
                oninput={(event) =>
                  (quickTitles = {
                    ...quickTitles,
                    [group.id]: event.currentTarget.value,
                  })}
                placeholder="New work item"
                value={quickTitles[group.id] ?? ""}
              />
              {#if (quickTitles[group.id] ?? "").trim()}
                <button
                  aria-label={`Add issue to ${group.name}`}
                  class="h-6 shrink-0 rounded bg-amber-400 px-2 text-[11px] font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
                  disabled={creatingGroupId === group.id}
                  type="submit"
                >
                  {creatingGroupId === group.id ? "Adding…" : "Add"}
                </button>
              {/if}
            </form>
          {/if}
        </div>
      {/if}
    </section>
  {/each}
</div>
