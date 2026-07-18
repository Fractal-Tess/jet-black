<script lang="ts">
import ArrowDown from "lucide-svelte/icons/arrow-down";
import ArrowUp from "lucide-svelte/icons/arrow-up";
import MessageSquare from "lucide-svelte/icons/message-square";
import type { IssueDisplayProperty } from "./display-options";
import IssuePropertyChips from "./IssuePropertyChips.svelte";
import type {
  Issue,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  issue,
  canReorder,
  dragging = false,
  index,
  members,
  onDragEnd,
  onDragStart,
  onMoveDown,
  onMoveToState,
  onMoveUp,
  onSelect,
  onUpdate,
  properties,
  selected = false,
  states,
  totalInGroup,
}: {
  issue: Issue;
  canReorder: boolean;
  dragging?: boolean;
  index: number;
  members: WorkspaceMember[];
  onDragEnd: () => void;
  onDragStart: (event: DragEvent, issue: Issue) => void;
  onMoveDown: () => Promise<void>;
  onMoveToState: (stateId: IssueState["_id"]) => Promise<void>;
  onMoveUp: () => Promise<void>;
  onSelect: (issue: Issue) => void;
  onUpdate: (input: UpdateIssueInput) => Promise<void>;
  properties: Record<IssueDisplayProperty, boolean>;
  selected?: boolean;
  states: IssueState[];
  totalInGroup: number;
} = $props();

const commentLabel = $derived(
  `${issue.commentCount} ${issue.commentCount === 1 ? "comment" : "comments"}`
);

function handleCardClick(event: MouseEvent) {
  const target = event.target as HTMLElement;

  // Interactive elements (chips, selects, reorder buttons) handle their own clicks.
  if (target.closest("button, select, a, input, [role='menuitem']")) {
    return;
  }

  onSelect(issue);
}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<article
  class="group/card relative cursor-pointer space-y-2 rounded-lg border bg-[#141515] p-3 shadow-sm shadow-black/20 transition hover:shadow-md hover:shadow-black/30 {selected
    ? 'border-amber-400/50'
    : 'border-white/[0.08] hover:border-white/20'} {dragging
    ? 'opacity-50'
    : ''}"
  draggable="true"
  onclick={handleCardClick}
  ondragend={onDragEnd}
  ondragstart={(event) => onDragStart(event, issue)}
>
  <button
    class="block w-full cursor-pointer text-left"
    onclick={() => onSelect(issue)}
    type="button"
  >
    {#if properties.key}
      <span class="font-mono text-[11px] text-zinc-500">
        {issue.identifier}
      </span>
    {/if}
    <span
      class="mt-0.5 line-clamp-2 block text-[13px] font-medium leading-5 text-zinc-100"
    >
      {issue.title}
    </span>
  </button>

  <div class="flex flex-wrap items-center gap-1.5">
    <IssuePropertyChips
      {issue}
      {members}
      {onMoveToState}
      {onUpdate}
      {properties}
      {states}
    />
    {#if issue.commentCount > 0}
      <span
        aria-label={commentLabel}
        class="flex h-5 items-center gap-1 rounded border border-white/10 px-1.5 text-[11px] text-zinc-500"
      >
        <MessageSquare class="size-3" />
        {issue.commentCount}
      </span>
    {/if}
  </div>

  {#if canReorder}
    <div
      class="absolute top-1.5 right-1.5 flex items-center gap-0.5 rounded-md border border-white/10 bg-[#1f2020] opacity-0 transition group-hover/card:opacity-100"
    >
      <button
        aria-label={`Reorder ${issue.identifier} up`}
        class="grid size-5 place-items-center rounded text-zinc-500 transition hover:bg-white/[0.06] hover:text-zinc-200 disabled:opacity-30"
        disabled={index === 0}
        onclick={onMoveUp}
        type="button"
      >
        <ArrowUp class="size-3" />
      </button>
      <button
        aria-label={`Reorder ${issue.identifier} down`}
        class="grid size-5 place-items-center rounded text-zinc-500 transition hover:bg-white/[0.06] hover:text-zinc-200 disabled:opacity-30"
        disabled={index === totalInGroup - 1}
        onclick={onMoveDown}
        type="button"
      >
        <ArrowDown class="size-3" />
      </button>
    </div>
  {/if}
</article>
