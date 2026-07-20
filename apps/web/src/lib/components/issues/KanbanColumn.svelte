<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Input } from "@workspace/ui/components/input";
import Plus from "lucide-svelte/icons/plus";
import { flip } from "svelte/animate";
import { receiveCard, sendCard } from "./card-transition";
import type { IssueDisplayProperty, IssueGroup } from "./display-options";
import KanbanCard from "./KanbanCard.svelte";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type { Issue, UpdateIssueInput, WorkspaceMember } from "./types";

let {
  canReorder,
  draggingIssueId,
  group,
  members,
  onDragEnd,
  onDragStart,
  onDrop,
  onDropOnCard,
  onOpenCreateIssue,
  onQuickCreate,
  onSelect,
  onUpdate,
  properties,
  selectedIssueId,
}: {
  canReorder: boolean;
  draggingIssueId: Issue["_id"] | null;
  group: IssueGroup;
  members: WorkspaceMember[];
  onDragEnd: () => void;
  onDragStart: (event: DragEvent, issue: Issue) => void;
  onDrop: (event: DragEvent, group: IssueGroup) => Promise<void>;
  onDropOnCard: (
    group: IssueGroup,
    index: number,
    edge: "after" | "before"
  ) => Promise<void>;
  onOpenCreateIssue?: (group: IssueGroup) => void;
  onQuickCreate?: (group: IssueGroup, title: string) => Promise<void>;
  onSelect: (issue: Issue) => void;
  onUpdate: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  properties: Record<IssueDisplayProperty, boolean>;
  selectedIssueId?: string;
} = $props();

let creating = $state(false);
let quickTitle = $state("");
let dragOver = $state(false);
let dropIndicator = $state<{
  edge: "after" | "before";
  issueId: Issue["_id"];
} | null>(null);

function handleCardDragOver(event: DragEvent, issue: Issue) {
  if (!(canReorder && draggingIssueId)) {
    return;
  }

  event.preventDefault();

  // Hovering the dragged card itself: no indicator, dropping is a no-op.
  if (draggingIssueId === issue._id) {
    dropIndicator = null;
    return;
  }

  const target = event.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  const edge = event.clientY < rect.top + rect.height / 2 ? "before" : "after";

  dropIndicator = { edge, issueId: issue._id };
}

function handleCardDragLeave(event: DragEvent, issue: Issue) {
  const related = event.relatedTarget as Node | null;
  const wrapper = event.currentTarget as HTMLElement;

  // Moving between children of the same card also fires dragleave; only
  // clear the indicator when the pointer actually leaves the card.
  if (related && wrapper.contains(related)) {
    return;
  }

  if (dropIndicator?.issueId === issue._id) {
    dropIndicator = null;
  }
}

async function handleCardDrop(event: DragEvent, issue: Issue, index: number) {
  if (!canReorder) {
    return;
  }

  // Dropping the card back onto itself keeps its position instead of
  // falling through to the column-level drop-at-end handler.
  if (draggingIssueId === issue._id) {
    event.preventDefault();
    event.stopPropagation();
    dragOver = false;
    return;
  }

  if (!dropIndicator || dropIndicator.issueId !== issue._id) {
    return;
  }

  const { edge } = dropIndicator;

  event.preventDefault();
  event.stopPropagation();
  dropIndicator = null;
  dragOver = false;
  await onDropOnCard(group, index, edge);
}

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
    ? 'bg-accent/50 ring-1 ring-primary/20'
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
          type={group.state.type}
        />
      {:else if group.priority}
        <PriorityIcon class="size-3.5" priority={group.priority} />
      {/if}
      <h3 class="truncate text-sm font-medium text-foreground">
        {group.name}
      </h3>
      <span class="text-sm tabular-nums text-muted-foreground">
        {group.issues.length}
      </span>
    </div>
    {#if onOpenCreateIssue || onQuickCreate}
      <Button
        aria-label={`New work item in ${group.name}`}
        class="size-5"
        onclick={() => {
          if (onOpenCreateIssue) {
            onOpenCreateIssue(group);
            return;
          }
          document.getElementById(`quick-add-${group.id}`)?.focus();
        }}
        size="icon-xs"
        variant="ghost"
      >
        <Plus class="size-3.5" />
      </Button>
    {/if}
  </div>

  {#if onQuickCreate}
    <form
      class="mx-2 mb-2 flex shrink-0 items-center gap-2 rounded-xl border border-border/60 bg-card/60 px-3 py-2 transition-colors hover:bg-card focus-within:border-border focus-within:bg-card"
      onsubmit={(event) => {
        event.preventDefault();
        submitQuickCreate();
      }}
    >
      <Plus class="size-3.5 shrink-0 text-muted-foreground" />
      <Input
        aria-label={`Quick issue title for ${group.name}`}
        bind:value={quickTitle}
        class="h-6 min-w-0 flex-1 border-0 bg-transparent px-1 text-sm shadow-none focus-visible:ring-0"
        id={`quick-add-${group.id}`}
        placeholder="New work item"
      />
      {#if quickTitle.trim()}
        <Button
          aria-label={`Add issue to ${group.name}`}
          disabled={creating}
          size="xs"
          type="submit"
        >
          {creating ? "Adding…" : "Add"}
        </Button>
      {/if}
    </form>
  {/if}

  <!-- pt-1.5 keeps the drop indicator of the first card (offset -5px) inside
       the overflow clip so it stays visible when dropping at the very top. -->
  <div
    class="flex flex-1 flex-col gap-2 overflow-x-hidden overflow-y-auto px-2 pt-1.5 pb-2"
  >
    {#each group.issues as issue, index (issue._id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        animate:flip={{ duration: 250 }}
        class="relative"
        in:receiveCard={{ key: issue._id }}
        ondragleave={(event) => handleCardDragLeave(event, issue)}
        ondragover={(event) => handleCardDragOver(event, issue)}
        ondrop={(event) => handleCardDrop(event, issue, index)}
        out:sendCard={{ key: issue._id }}
      >
        {#if dropIndicator?.issueId === issue._id}
          <div
            aria-hidden="true"
            class="pointer-events-none absolute inset-x-1 z-10 h-0.5 rounded-full bg-primary {dropIndicator.edge ===
            'before'
              ? '-top-[5px]'
              : '-bottom-[5px]'}"
          ></div>
        {/if}
        <KanbanCard
          dragging={draggingIssueId === issue._id}
          {issue}
          {members}
          {onDragEnd}
          {onDragStart}
          {onSelect}
          onUpdate={(input) => onUpdate(issue, input)}
          {properties}
          selected={selectedIssueId === issue._id}
        />
      </div>
    {:else}
      {#if !onQuickCreate}
        <div
          class="rounded-xl border border-dashed border-border px-3 py-8 text-center text-xs text-muted-foreground"
        >
          No work items
        </div>
      {/if}
    {/each}
  </div>
</section>
