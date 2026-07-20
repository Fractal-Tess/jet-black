<script lang="ts">
import Box from "lucide-svelte/icons/box";
import Check from "lucide-svelte/icons/check";
import ChevronDown from "lucide-svelte/icons/chevron-down";
import CircleDot from "lucide-svelte/icons/circle-dot";
import Plus from "lucide-svelte/icons/plus";
import Signal from "lucide-svelte/icons/signal";
import User from "lucide-svelte/icons/user";
import Users from "lucide-svelte/icons/users";
import { priorityOptions } from "./issuePropertyOptions";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type {
  Issue,
  IssuePriority,
  IssueState,
  ProjectModuleRecord,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  issue,
  members,
  modules = [],
  states,
  openDropdown,
  onToggleDropdown,
  onCloseDropdown,
  onUpdateIssue,
  onCreateModuleForIssue,
}: {
  issue: Issue;
  members: WorkspaceMember[];
  modules?: ProjectModuleRecord[];
  states: IssueState[];
  openDropdown: string | null;
  onToggleDropdown: (name: string) => void;
  onCloseDropdown: () => void;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
  onCreateModuleForIssue: (name: string) => Promise<void>;
} = $props();

const currentAssignee = $derived(
  issue.assigneeUserId
    ? (members.find((m) => m.id === issue.assigneeUserId) ?? null)
    : null
);

const currentReporter = $derived(
  members.find((m) => m.id === issue.createdByUserId) ?? null
);

let memberSearch = $state("");

function toggleMemberDropdown(name: "assignee" | "reporter") {
  memberSearch = "";
  onToggleDropdown(name);
}

const filteredMembers = $derived.by(() => {
  const query = memberSearch.trim().toLowerCase();

  if (!query) {
    return members;
  }

  return members.filter(
    (member) =>
      member.name.toLowerCase().includes(query) ||
      member.email.toLowerCase().includes(query)
  );
});

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

function handleReporterChange(newReporterId: string) {
  onUpdateIssue({ createdByUserId: newReporterId });
  onCloseDropdown();
}

const currentModule = $derived(
  modules.find((m) => m._id === issue.moduleId) ?? null
);

let moduleSearch = $state("");

function toggleModuleDropdown() {
  moduleSearch = "";
  onToggleDropdown("module");
}

const filteredModules = $derived.by(() => {
  const query = moduleSearch.trim().toLowerCase();

  if (!query) {
    return modules;
  }

  return modules.filter((projectModule) =>
    projectModule.name.toLowerCase().includes(query)
  );
});

function handleModuleChange(moduleId: ProjectModuleRecord["_id"] | null) {
  onUpdateIssue({ moduleId });
  onCloseDropdown();
}

let creatingModule = $state(false);

const moduleCreateName = $derived.by(() => {
  const name = moduleSearch.trim();
  if (!name) {
    return null;
  }
  const exists = modules.some(
    (projectModule) => projectModule.name.toLowerCase() === name.toLowerCase()
  );
  return exists ? null : name;
});

async function handleCreateModule() {
  if (!moduleCreateName || creatingModule) {
    return;
  }
  creatingModule = true;
  try {
    await onCreateModuleForIssue(moduleCreateName);
    onCloseDropdown();
  } finally {
    creatingModule = false;
  }
}
</script>

{#snippet memberAvatar(member: WorkspaceMember)}
	<span class="grid size-5 shrink-0 place-items-center overflow-hidden rounded-full bg-primary text-meta font-medium text-primary-foreground">
		{#if member.image}
			<img alt="" class="size-full object-cover" src={member.image} />
		{:else}
			{member.name.slice(0, 2).toUpperCase()}
		{/if}
	</span>
{/snippet}

<!-- State -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
		<CircleDot class="size-4" />
		<span>State</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-lg px-2 py-1 text-sm text-foreground transition-colors hover:bg-muted"
			onclick={() => onToggleDropdown("state")}
			type="button"
		>
			{#if currentState}<StateTypeIcon class="size-3.5" type={currentState.type} />{/if}
			{currentState?.name ?? "No state"}
			<ChevronDown class="size-3 text-muted-foreground" />
		</button>
		{#if openDropdown === "state"}
			<div class="absolute left-0 top-8 z-50 w-48 rounded-lg border border-border bg-popover py-1 text-popover-foreground shadow-md">
				{#each states as state (state._id)}
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {state._id === issue.stateId ? 'text-foreground' : 'text-muted-foreground'}"
						onclick={() => handleStateChange(state._id)}
						type="button"
					>
						<StateTypeIcon class="size-3.5" type={state.type} />
						<span class="flex-1">{state.name}</span>
						{#if state._id === issue.stateId}
							<Check class="size-3.5 text-primary" />
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<!-- Assignees -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
		<Users class="size-4" />
		<span>Assignees</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-lg px-2 py-1 text-sm transition-colors hover:bg-muted {currentAssignee ? 'text-foreground' : 'text-muted-foreground'}"
			onclick={() => toggleMemberDropdown("assignee")}
			type="button"
		>
			{#if currentAssignee}
				{@render memberAvatar(currentAssignee)}
				{currentAssignee.name}
			{:else}
				Add assignees
			{/if}
			<ChevronDown class="size-3 text-muted-foreground" />
		</button>
		{#if openDropdown === "assignee"}
			<div class="absolute left-0 top-8 z-50 w-56 rounded-lg border border-border bg-popover py-1 text-popover-foreground shadow-md">
				<div class="border-b border-border px-3 pb-1.5 pt-0.5">
					<!-- svelte-ignore a11y_autofocus -->
					<input
						aria-label="Search members"
						autofocus
						bind:value={memberSearch}
						class="w-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
						placeholder="Search members…"
						type="text"
					/>
				</div>
				<div class="max-h-56 overflow-y-auto">
				<!-- Unassign option -->
				<button
					class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {!issue.assigneeUserId ? 'text-foreground' : 'text-muted-foreground'}"
					onclick={() => handleAssigneeChange(null)}
					type="button"
				>
					<span class="grid size-5 place-items-center rounded-full border border-dashed border-border text-meta text-muted-foreground">?</span>
					<span class="flex-1">Unassigned</span>
					{#if !issue.assigneeUserId}
						<Check class="size-3.5 text-primary" />
					{/if}
				</button>
				{#if filteredMembers.length === 0}
					<p class="px-3 py-2 text-sm text-muted-foreground">No members found</p>
				{/if}
				{#each filteredMembers as member (member.id)}
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {issue.assigneeUserId === member.id ? 'text-foreground' : 'text-muted-foreground'}"
						onclick={() => handleAssigneeChange(member.id)}
						type="button"
					>
						{@render memberAvatar(member)}
						<span class="flex-1 truncate">{member.name}</span>
						{#if issue.assigneeUserId === member.id}
							<Check class="size-3.5 text-primary" />
						{/if}
					</button>
				{/each}
				</div>
			</div>
		{/if}
	</div>
</div>

<!-- Priority -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
		<Signal class="size-4" />
		<span>Priority</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-lg px-2 py-1 text-sm text-foreground transition-colors hover:bg-muted"
			onclick={() => onToggleDropdown("priority")}
			type="button"
		>
			<PriorityIcon class="size-3.5" priority={issue.priority} />
			{currentPriority.label}
			<ChevronDown class="size-3 text-muted-foreground" />
		</button>
		{#if openDropdown === "priority"}
			<div class="absolute left-0 top-8 z-50 w-48 rounded-lg border border-border bg-popover py-1 text-popover-foreground shadow-md">
				{#each priorityOptions as opt (opt.value)}
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {opt.value === issue.priority ? 'text-foreground' : 'text-muted-foreground'}"
						onclick={() => handlePriorityChange(opt.value)}
						type="button"
					>
						<PriorityIcon class="size-3.5" priority={opt.value} />
						<span class="flex-1">{opt.label}</span>
						{#if opt.value === issue.priority}
							<Check class="size-3.5 text-primary" />
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<!-- Module -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
		<Box class="size-4" />
		<span>Module</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-lg px-2 py-1 text-sm transition-colors hover:bg-muted {currentModule ? 'text-foreground' : 'text-muted-foreground'}"
			onclick={toggleModuleDropdown}
			type="button"
		>
			{currentModule?.name ?? "No module"}
			<ChevronDown class="size-3 text-muted-foreground" />
		</button>
		{#if openDropdown === "module"}
			<div class="absolute left-0 top-8 z-50 w-56 rounded-lg border border-border bg-popover py-1 text-popover-foreground shadow-md">
				<div class="border-b border-border px-3 pb-1.5 pt-0.5">
					<!-- svelte-ignore a11y_autofocus -->
					<input
						aria-label="Search modules"
						autofocus
						bind:value={moduleSearch}
						class="w-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
						placeholder="Search modules…"
						type="text"
					/>
				</div>
				<div class="max-h-56 overflow-y-auto">
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {issue.moduleId ? 'text-muted-foreground' : 'text-foreground'}"
						onclick={() => handleModuleChange(null)}
						type="button"
					>
						<span class="flex-1">No module</span>
						{#if !issue.moduleId}
							<Check class="size-3.5 text-primary" />
						{/if}
					</button>
					{#if filteredModules.length === 0 && !moduleCreateName}
						<p class="px-3 py-2 text-sm text-muted-foreground">No modules found</p>
					{/if}
					{#each filteredModules as projectModule (projectModule._id)}
						<button
							class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {issue.moduleId === projectModule._id ? 'text-foreground' : 'text-muted-foreground'}"
							onclick={() => handleModuleChange(projectModule._id)}
							type="button"
						>
							<Box class="size-3.5" />
							<span class="flex-1 truncate">{projectModule.name}</span>
							{#if issue.moduleId === projectModule._id}
								<Check class="size-3.5 text-primary" />
							{/if}
						</button>
					{/each}
					{#if moduleCreateName}
						<button
							class="flex w-full items-center gap-2.5 border-t border-border px-3 py-2 text-left text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
							disabled={creatingModule}
							onclick={handleCreateModule}
							type="button"
						>
							<Plus class="size-3.5" />
							<span class="flex-1 truncate">
								{creatingModule ? "Creating…" : `Create "${moduleCreateName}"`}
							</span>
						</button>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</div>

<!-- Reported by -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
		<User class="size-4" />
		<span>Reported by</span>
	</div>
	<div class="relative" data-dropdown>
		<button
			class="flex items-center gap-2 rounded-lg px-2 py-1 text-sm transition-colors hover:bg-muted {currentReporter ? 'text-foreground' : 'text-muted-foreground'}"
			onclick={() => toggleMemberDropdown("reporter")}
			type="button"
		>
			{#if currentReporter}
				{@render memberAvatar(currentReporter)}
				{currentReporter.name}
			{:else}
				Add reporter
			{/if}
			<ChevronDown class="size-3 text-muted-foreground" />
		</button>
		{#if openDropdown === "reporter"}
			<div class="absolute left-0 top-8 z-50 w-56 rounded-lg border border-border bg-popover py-1 text-popover-foreground shadow-md">
				<div class="border-b border-border px-3 pb-1.5 pt-0.5">
					<!-- svelte-ignore a11y_autofocus -->
					<input
						aria-label="Search members"
						autofocus
						bind:value={memberSearch}
						class="w-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
						placeholder="Search members…"
						type="text"
					/>
				</div>
				<div class="max-h-56 overflow-y-auto">
				{#if filteredMembers.length === 0}
					<p class="px-3 py-2 text-sm text-muted-foreground">No members found</p>
				{/if}
				{#each filteredMembers as member (member.id)}
					<button
						class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors hover:bg-muted {issue.createdByUserId === member.id ? 'text-foreground' : 'text-muted-foreground'}"
						onclick={() => handleReporterChange(member.id)}
						type="button"
					>
						{@render memberAvatar(member)}
						<span class="flex-1 truncate">{member.name}</span>
						{#if issue.createdByUserId === member.id}
							<Check class="size-3.5 text-primary" />
						{/if}
					</button>
				{/each}
				</div>
			</div>
		{/if}
	</div>
</div>
