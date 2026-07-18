<script lang="ts">
import Check from "lucide-svelte/icons/check";
import ChevronDown from "lucide-svelte/icons/chevron-down";
import CircleDot from "lucide-svelte/icons/circle-dot";
import Signal from "lucide-svelte/icons/signal";
import User from "lucide-svelte/icons/user";
import Users from "lucide-svelte/icons/users";
import { priorityOptions } from "./issuePropertyOptions";
import type {
  Issue,
  IssuePriority,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  issue,
  members,
  states,
  openDropdown,
  onToggleDropdown,
  onCloseDropdown,
  onUpdateIssue,
}: {
  issue: Issue;
  members: WorkspaceMember[];
  states: IssueState[];
  openDropdown: string | null;
  onToggleDropdown: (name: string) => void;
  onCloseDropdown: () => void;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
} = $props();

const currentAssignee = $derived(
  issue.assigneeUserId
    ? (members.find((m) => m.id === issue.assigneeUserId) ?? null)
    : null
);

const currentState = $derived(states.find((s) => s._id === issue.stateId));
const currentPriority = $derived(
  priorityOptions.find((p) => p.value === issue.priority) ?? priorityOptions[4]
);

function handleStateChange(newStateId: IssueState["_id"]) {
  onUpdateIssue({ stateId: newStateId });
  onCloseDropdown();
}

function handlePriorityChange(newPriority: IssuePriority) {
  onUpdateIssue({ priority: newPriority });
  onCloseDropdown();
}

function handleAssigneeChange(newAssigneeId: string | null) {
  onUpdateIssue({ assigneeUserId: newAssigneeId });
  onCloseDropdown();
}
</script>

<!-- State -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-zinc-500">
		<CircleDot class="size-4" />
		<span>State</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-md px-2 py-1 text-sm text-zinc-300 transition hover:bg-white/[0.06]"
			onclick={() => onToggleDropdown("state")}
			type="button"
		>
			<span
				class="size-2.5 rounded-full"
				style:background-color={currentState?.color ?? "#71717a"}
			></span>
			{currentState?.name ?? "No state"}
			<ChevronDown class="size-3 text-zinc-500" />
		</button>
		{#if openDropdown === "state"}
			<div class="absolute left-0 top-8 z-50 w-48 rounded-lg border border-white/10 bg-[#1a1b1b] py-1 shadow-xl">
				{#each states as state (state._id)}
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition hover:bg-white/[0.06] {state._id === issue.stateId ? 'text-zinc-100' : 'text-zinc-400'}"
						onclick={() => handleStateChange(state._id)}
						type="button"
					>
						<span
							class="size-2.5 rounded-full"
							style:background-color={state.color}
						></span>
						<span class="flex-1">{state.name}</span>
						{#if state._id === issue.stateId}
							<Check class="size-3.5 text-amber-400" />
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<!-- Assignees -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-zinc-500">
		<Users class="size-4" />
		<span>Assignees</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-md px-2 py-1 text-sm transition hover:bg-white/[0.06] {currentAssignee ? 'text-zinc-300' : 'text-zinc-500'}"
			onclick={() => onToggleDropdown("assignee")}
			type="button"
		>
			{#if currentAssignee}
				<span class="grid size-5 place-items-center rounded-full bg-zinc-700 text-[10px] font-medium text-zinc-300">
					{currentAssignee.name.slice(0, 2).toUpperCase()}
				</span>
				{currentAssignee.name}
			{:else}
				Add assignees
			{/if}
			<ChevronDown class="size-3 text-zinc-500" />
		</button>
		{#if openDropdown === "assignee"}
			<div class="absolute left-0 top-8 z-50 w-56 rounded-lg border border-white/10 bg-[#1a1b1b] py-1 shadow-xl">
				<!-- Unassign option -->
				<button
					class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition hover:bg-white/[0.06] {!issue.assigneeUserId ? 'text-zinc-100' : 'text-zinc-400'}"
					onclick={() => handleAssigneeChange(null)}
					type="button"
				>
					<span class="grid size-5 place-items-center rounded-full border border-dashed border-zinc-600 text-[10px] text-zinc-500">?</span>
					<span class="flex-1">Unassigned</span>
					{#if !issue.assigneeUserId}
						<Check class="size-3.5 text-amber-400" />
					{/if}
				</button>
				{#each members as member (member.id)}
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition hover:bg-white/[0.06] {issue.assigneeUserId === member.id ? 'text-zinc-100' : 'text-zinc-400'}"
						onclick={() => handleAssigneeChange(member.id)}
						type="button"
					>
						<span class="grid size-5 place-items-center rounded-full bg-zinc-700 text-[10px] font-medium text-zinc-300">
							{member.name.slice(0, 2).toUpperCase()}
						</span>
						<span class="flex-1 truncate">{member.name}</span>
						{#if issue.assigneeUserId === member.id}
							<Check class="size-3.5 text-amber-400" />
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<!-- Priority -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-zinc-500">
		<Signal class="size-4" />
		<span>Priority</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-md px-2 py-1 text-sm text-zinc-300 transition hover:bg-white/[0.06]"
			onclick={() => onToggleDropdown("priority")}
			type="button"
		>
			<span
				class="size-2.5 rounded-full"
				style:background-color={currentPriority.color}
			></span>
			{currentPriority.label}
			<ChevronDown class="size-3 text-zinc-500" />
		</button>
		{#if openDropdown === "priority"}
			<div class="absolute left-0 top-8 z-50 w-48 rounded-lg border border-white/10 bg-[#1a1b1b] py-1 shadow-xl">
				{#each priorityOptions as opt (opt.value)}
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition hover:bg-white/[0.06] {opt.value === issue.priority ? 'text-zinc-100' : 'text-zinc-400'}"
						onclick={() => handlePriorityChange(opt.value)}
						type="button"
					>
						<span
							class="size-2.5 rounded-full"
							style:background-color={opt.color}
						></span>
						<span class="flex-1">{opt.label}</span>
						{#if opt.value === issue.priority}
							<Check class="size-3.5 text-amber-400" />
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<!-- Created by -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-zinc-500">
		<User class="size-4" />
		<span>Created by</span>
	</div>
	<div class="flex items-center gap-2">
		<span class="grid size-5 place-items-center rounded-full bg-zinc-700 text-[10px] font-medium text-zinc-300">
			{(issue.assigneeUserId ?? issue.createdByUserId).slice(0, 2).toUpperCase()}
		</span>
		<span class="text-sm text-zinc-300">
			{issue.createdByUserId.slice(0, 8)}
		</span>
	</div>
</div>
