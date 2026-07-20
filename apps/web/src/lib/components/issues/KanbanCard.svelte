<script lang="ts">
import MessageSquare from "lucide-svelte/icons/message-square";
import type { IssueDisplayProperty } from "./display-options";
import IssuePropertyChips from "./IssuePropertyChips.svelte";
import type { Issue, UpdateIssueInput, WorkspaceMember } from "./types";

let {
  issue,
  dragging = false,
  members,
  onDragEnd,
  onDragStart,
  onSelect,
  onUpdate,
  properties,
  selected = false,
}: {
  issue: Issue;
  dragging?: boolean;
  members: WorkspaceMember[];
  onDragEnd: () => void;
  onDragStart: (event: DragEvent, issue: Issue) => void;
  onSelect: (issue: Issue) => void;
  onUpdate: (input: UpdateIssueInput) => Promise<void>;
  properties: Record<IssueDisplayProperty, boolean>;
  selected?: boolean;
} = $props();

const commentLabel = $derived(
  `${issue.commentCount} ${issue.commentCount === 1 ? "comment" : "comments"}`
);

function handleCardClick(event: MouseEvent) {
  const target = event.target as HTMLElement;

  // Interactive elements (chips, selects, links) handle their own clicks.
  if (target.closest("button, select, a, input, [role='menuitem']")) {
    return;
  }

  onSelect(issue);
}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<article
  class="group/card relative cursor-pointer space-y-2 rounded-xl border bg-card p-3 shadow-sm transition-all hover:border-input hover:shadow-md {selected
    ? 'border-primary/50 ring-1 ring-primary/20'
    : 'border-border'} {dragging
    ? 'opacity-50'
    : ''}"
  draggable="true"
  onclick={handleCardClick}
  ondragend={onDragEnd}
  ondragstart={(event) => onDragStart(event, issue)}
>
  <button
    class="block w-full cursor-pointer text-left focus:outline-none"
    onclick={() => onSelect(issue)}
    type="button"
  >
    {#if properties.key}
      <span class="font-mono text-meta text-muted-foreground">
        {issue.identifier}
      </span>
    {/if}
    <span
      class="mt-0.5 line-clamp-2 block text-sm font-medium leading-5 text-card-foreground"
    >
      {issue.title}
    </span>
  </button>

  <div class="flex flex-wrap items-center gap-1.5">
    <IssuePropertyChips
      {issue}
      {members}
      {onUpdate}
      {properties}
    />
    {#if issue.commentCount > 0}
      <span
        aria-label={commentLabel}
        class="flex h-6 items-center gap-1.5 rounded-md border border-border px-2 text-xs text-muted-foreground"
      >
        <MessageSquare class="size-3.5" />
        {issue.commentCount}
      </span>
    {/if}
  </div>
</article>
