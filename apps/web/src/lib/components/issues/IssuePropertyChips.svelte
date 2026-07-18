<script lang="ts">
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@workspace/ui/components/dropdown-menu";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import ChevronDown from "lucide-svelte/icons/chevron-down";
import CircleUserRound from "lucide-svelte/icons/circle-user-round";
import {
  type IssueDisplayProperty,
  PRIORITY_LABELS,
  PRIORITY_ORDER,
} from "./display-options";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type {
  Issue,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  issue,
  members,
  onMoveToState,
  onUpdate,
  properties,
  states,
}: {
  issue: Issue;
  members: WorkspaceMember[];
  onMoveToState: (stateId: IssueState["_id"]) => Promise<void>;
  onUpdate: (input: UpdateIssueInput) => Promise<void>;
  properties: Record<IssueDisplayProperty, boolean>;
  states: IssueState[];
} = $props();

const MAX_VISIBLE_LABELS = 2;
const WHITESPACE_PATTERN = /\s+/;

const chipClass =
  "flex h-5 shrink-0 cursor-pointer items-center gap-1 rounded border border-white/10 px-1.5 text-[11px] text-zinc-400 transition hover:bg-white/[0.06] hover:text-zinc-200";

const currentState = $derived(
  states.find((state) => state._id === issue.stateId)
);

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

<div class="flex flex-wrap items-center gap-1.5">
  {#if properties.state && currentState}
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={`Move ${issue.identifier}`}
        class={chipClass}
        onclick={(event: MouseEvent) => event.stopPropagation()}
        title={`State: ${currentState.name}`}
      >
        <StateTypeIcon
          class="size-3"
          color={currentState.color}
          type={currentState.type}
        />
        <span class="max-w-24 truncate">{currentState.name}</span>
        <ChevronDown class="size-2.5 text-zinc-600" />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        class="w-40 border border-white/10 bg-[#151616]"
      >
        {#each states as state (state._id)}
          <DropdownMenuItem
            class="gap-2 text-xs text-zinc-300"
            onclick={() => onMoveToState(state._id)}
          >
            <StateTypeIcon
              class="size-3.5"
              color={state.color}
              type={state.type}
            />
            {state.name}
          </DropdownMenuItem>
        {/each}
      </DropdownMenuContent>
    </DropdownMenu>
  {/if}

  {#if properties.priority}
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={`Priority ${PRIORITY_LABELS[issue.priority]} for ${issue.identifier}`}
        class={chipClass}
        onclick={(event: MouseEvent) => event.stopPropagation()}
        title={`Priority: ${PRIORITY_LABELS[issue.priority]}`}
      >
        <PriorityIcon class="size-3" priority={issue.priority} />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        class="w-36 border border-white/10 bg-[#151616]"
      >
        {#each PRIORITY_ORDER as priority (priority)}
          <DropdownMenuItem
            class="gap-2 text-xs text-zinc-300"
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
          <span
            class="grid size-3.5 place-items-center rounded-full bg-amber-400/90 text-[8px] font-semibold text-black"
          >
            {initials(assignee.name)}
          </span>
        {:else}
          <CircleUserRound class="size-3 text-zinc-500" />
        {/if}
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        class="w-44 border border-white/10 bg-[#151616]"
      >
        {#each members as member (member.id)}
          <DropdownMenuItem
            class="gap-2 text-xs text-zinc-300"
            onclick={() => onUpdate({ assigneeUserId: member.id })}
          >
            <span
              class="grid size-4 place-items-center rounded-full bg-amber-400/90 text-[8px] font-semibold text-black"
            >
              {initials(member.name)}
            </span>
            <span class="truncate">{member.name}</span>
          </DropdownMenuItem>
        {/each}
        {#if issue.assigneeUserId}
          <DropdownMenuSeparator class="bg-white/10" />
          <DropdownMenuItem
            class="gap-2 text-xs text-zinc-400"
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
          class="size-1.5 rounded-full"
          style:background-color={label.color}
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
      class="{chipClass} {isOverdue ? 'text-red-400 hover:text-red-300' : ''}"
      title="Due date"
    >
      <CalendarDays class="size-3" />
      {formatDate(issue.targetDate)}
    </span>
  {/if}
</div>
