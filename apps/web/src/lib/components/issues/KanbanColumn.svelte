<script lang="ts">
import Plus from "lucide-svelte/icons/plus";
import { flip } from "svelte/animate";
import { receiveCard, sendCard } from "./card-transition";
import type { IssueDisplayProperty, IssueGroup } from "./display-options";
import KanbanCard from "./KanbanCard.svelte";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type {
  Issue,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  canReorder,
  draggingIssueId,
  group,
  members,
  onDragEnd,
  onDragStart,
  onDrop,
  onMoveToState,
  onQuickCreate,
  onReorder,
  onSelect,
  onUpdate,
  properties,
  selectedIssueId,
  states,
}: {
  canReorder: boolean;
  draggingIssueId: Issue["_id"] | null;
  group: IssueGroup;
  members: WorkspaceMember[];
  onDragEnd: () => void;
  onDragStart: (event: DragEvent, issue: Issue) => void;
  onDrop: (event: DragEvent, group: IssueGroup) => Promise<void>;
  onMoveToState: (issue: Issue, stateId: IssueState["_id"]) => Promise<void>;
  onQuickCreate?: (group: IssueGroup, title: string) => Promise<void>;
  onReorder: (
    issue: Issue,
    group: IssueGroup,
    direction: "down" | "up",
    index: number
  ) => Promise<void>;
  onSelect: (issue: Issue) => void;
  onUpdate: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  properties: Record<IssueDisplayProperty, boolean>;
  selectedIssueId?: string;
  states: IssueState[];
} = $props();

let creating = $state(false);
let quickTitle = $state("");
let dragOver = $state(false);

async function submitQuickCreate() {
  const title = quickTitle.trim();

  if (!(title && onQuickCreate)) {
    return;
  }

  creating = true;

  try {
    await onQuickCreate(group, title);
    quickTitle = "";
  } finally {
    creating = false;
  }
}
</script>

<section
  aria-label={`${group.name} column`}
  class="flex w-[324px] shrink-0 flex-col rounded-lg transition {dragOver &&
  draggingIssueId
    ? 'bg-white/[0.04] ring-1 ring-amber-400/20'
    : ''}"
  ondragleave={() => (dragOver = false)}
  ondragover={(event) => {
    event.preventDefault();
    dragOver = true;
  }}
  ondrop={async (event) => {
    dragOver = false;
    await onDrop(event, group);
  }}
>
  <div
    class="sticky top-0 z-10 flex items-center justify-between rounded-t-lg px-3 py-2.5"
  >
    <div class="flex min-w-0 items-center gap-2">
      {#if group.state}
        <StateTypeIcon
          class="size-3.5"
          color={group.state.color}
          type={group.state.type}
        />
      {:else if group.priority}
        <PriorityIcon class="size-3.5" priority={group.priority} />
      {/if}
      <h3 class="truncate text-sm font-medium text-zinc-200">
        {group.name}
      </h3>
      <span class="text-sm tabular-nums text-zinc-500">
        {group.issues.length}
      </span>
    </div>
    {#if onQuickCreate}
      <button
        aria-label={`New work item in ${group.name}`}
        class="grid size-5 place-items-center rounded text-zinc-500 transition hover:bg-white/[0.06] hover:text-zinc-200"
        onclick={() =>
          document
            .getElementById(`quick-add-${group.id}`)
            ?.focus()}
        type="button"
      >
        <Plus class="size-3.5" />
      </button>
    {/if}
  </div>

  <div class="flex flex-1 flex-col gap-2 overflow-x-hidden overflow-y-auto px-2 pb-2">
    {#each group.issues as issue, index (issue._id)}
      <div
        animate:flip={{ duration: 250 }}
        in:receiveCard={{ key: issue._id }}
        out:sendCard={{ key: issue._id }}
      >
        <KanbanCard
          {canReorder}
          dragging={draggingIssueId === issue._id}
          {index}
          {issue}
          {members}
          {onDragEnd}
          {onDragStart}
          onMoveDown={() => onReorder(issue, group, "down", index)}
          onMoveToState={(stateId) => onMoveToState(issue, stateId)}
          onMoveUp={() => onReorder(issue, group, "up", index)}
          {onSelect}
          onUpdate={(input) => onUpdate(issue, input)}
          {properties}
          selected={selectedIssueId === issue._id}
          {states}
          totalInGroup={group.issues.length}
        />
      </div>
    {:else}
      {#if !onQuickCreate}
        <div
          class="rounded-lg border border-dashed border-white/[0.06] px-3 py-8 text-center text-xs text-zinc-700"
        >
          No work items
        </div>
      {/if}
    {/each}

    {#if onQuickCreate}
      <form
        class="flex items-center gap-2 rounded-lg border border-transparent px-3 py-2 transition focus-within:border-white/10 focus-within:bg-[#141515]"
        onsubmit={(event) => {
          event.preventDefault();
          submitQuickCreate();
        }}
      >
        <Plus class="size-3.5 shrink-0 text-zinc-600" />
        <input
          aria-label={`Quick issue title for ${group.name}`}
          bind:value={quickTitle}
          class="min-w-0 flex-1 bg-transparent text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600"
          id={`quick-add-${group.id}`}
          placeholder="New work item"
        />
        {#if quickTitle.trim()}
          <button
            aria-label={`Add issue to ${group.name}`}
            class="h-6 shrink-0 rounded bg-amber-400 px-2 text-[11px] font-semibold text-black transition hover:bg-amber-300 disabled:opacity-50"
            disabled={creating}
            type="submit"
          >
            {creating ? "Adding…" : "Add"}
          </button>
        {/if}
      </form>
    {/if}
  </div>
</section>
