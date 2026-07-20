<script lang="ts">
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@workspace/ui/components/dropdown-menu";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import CircleUserRound from "lucide-svelte/icons/circle-user-round";
import {
  type IssueDisplayProperty,
  PRIORITY_LABELS,
  PRIORITY_ORDER,
} from "./display-options";
import PriorityIcon from "./PriorityIcon.svelte";
import type { Issue, UpdateIssueInput, WorkspaceMember } from "./types";

let {
  issue,
  members,
  onUpdate,
  properties,
}: {
  issue: Issue;
  members: WorkspaceMember[];
  onUpdate: (input: UpdateIssueInput) => Promise<void>;
  properties: Record<IssueDisplayProperty, boolean>;
} = $props();

const MAX_VISIBLE_LABELS = 2;
const WHITESPACE_PATTERN = /\s+/;

const chipClass =
  "flex h-6 shrink-0 cursor-pointer items-center gap-1.5 rounded-md border border-border px-2 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground";

const assignee = $derived(
  issue.assigneeUserId
    ? members.find((member) => member.id === issue.assigneeUserId)
    : undefined
);

const visibleLabels = $derived(issue.labels.slice(0, MAX_VISIBLE_LABELS));
const hiddenLabelCount = $derived(issue.labels.length - visibleLabels.length);

function initials(name: string) {
  const parts = name.trim().split(WHITESPACE_PATTERN);
  const first = parts[0]?.[0] ?? "?";
  const last = parts.length > 1 ? parts.at(-1)?.[0] : "";

  return `${first}${last ?? ""}`.toUpperCase();
}

function formatDate(date: string) {
  return new Date(date).toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
  });
}

const isOverdue = $derived(
  Boolean(
    issue.targetDate &&
      !issue.completedAt &&
      new Date(issue.targetDate).getTime() < Date.now()
  )
);
</script>

{#snippet memberAvatar(member: WorkspaceMember, sizeClass: string)}
  <span
    class="grid {sizeClass} shrink-0 place-items-center overflow-hidden rounded-full bg-primary text-meta font-semibold text-primary-foreground"
  >
    {#if member.image}
      <img alt="" class="size-full object-cover" src={member.image} />
    {:else}
      {initials(member.name)}
    {/if}
  </span>
{/snippet}

<div class="flex flex-wrap items-center gap-1.5">
  {#if properties.priority}
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={`Priority ${PRIORITY_LABELS[issue.priority]} for ${issue.identifier}`}
        class={chipClass}
        onclick={(event: MouseEvent) => event.stopPropagation()}
        title={`Priority: ${PRIORITY_LABELS[issue.priority]}`}
      >
        <PriorityIcon class="size-3.5" priority={issue.priority} />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        class="w-36"
      >
        {#each PRIORITY_ORDER as priority (priority)}
          <DropdownMenuItem
            class="gap-2 text-xs"
            onclick={() => onUpdate({ priority })}
          >
            <PriorityIcon class="size-3.5" {priority} />
            {PRIORITY_LABELS[priority]}
          </DropdownMenuItem>
        {/each}
      </DropdownMenuContent>
    </DropdownMenu>
  {/if}

  {#if properties.assignee}
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={`Assignee for ${issue.identifier}`}
        class={chipClass}
        onclick={(event: MouseEvent) => event.stopPropagation()}
        title={assignee ? `Assignee: ${assignee.name}` : "Unassigned"}
      >
        {#if assignee}
          {@render memberAvatar(assignee, "size-4")}
        {:else}
          <CircleUserRound class="size-3.5 text-muted-foreground" />
        {/if}
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        class="w-44"
      >
        {#each members as member (member.id)}
          <DropdownMenuItem
            class="gap-2 text-xs"
            onclick={() => onUpdate({ assigneeUserId: member.id })}
          >
            {@render memberAvatar(member, "size-4")}
            <span class="truncate">{member.name}</span>
          </DropdownMenuItem>
        {/each}
        {#if issue.assigneeUserId}
          <DropdownMenuSeparator />
          <DropdownMenuItem
            class="gap-2 text-xs text-muted-foreground"
            onclick={() => onUpdate({ assigneeUserId: null })}
          >
            <CircleUserRound class="size-3.5" />
            Unassigned
          </DropdownMenuItem>
        {/if}
      </DropdownMenuContent>
    </DropdownMenu>
  {/if}

  {#if properties.labels}
    {#each visibleLabels as label (label._id)}
      <span class={chipClass} title={label.name}>
        <span
          class="size-2 rounded-full bg-primary"
        ></span>
        <span class="max-w-20 truncate">{label.name}</span>
      </span>
    {/each}
    {#if hiddenLabelCount > 0}
      <span class={chipClass}>+{hiddenLabelCount}</span>
    {/if}
  {/if}

  {#if properties.dueDate && issue.targetDate}
    <span
      class="{chipClass} {isOverdue ? 'text-destructive hover:text-destructive' : ''}"
      title="Due date"
    >
      <CalendarDays class="size-3.5" />
      {formatDate(issue.targetDate)}
    </span>
  {/if}
</div>
