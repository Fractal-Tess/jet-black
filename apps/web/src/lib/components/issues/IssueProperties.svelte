<script lang="ts">
import IssueCoreProperties from "./IssueCoreProperties.svelte";
import IssueLabelProperty from "./IssueLabelProperty.svelte";
import IssueScheduleProperties from "./IssueScheduleProperties.svelte";
import type {
  CreateLabelInput,
  Issue,
  IssueLabel,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  issue,
  labels,
  members,
  states,
  onCreateLabel,
  onToggleLabel,
  onUpdateIssue,
}: {
  issue: Issue;
  labels: IssueLabel[];
  members: WorkspaceMember[];
  states: IssueState[];
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
} = $props();

let openDropdown: string | null = $state(null);

function toggleDropdown(name: string) {
  openDropdown = openDropdown === name ? null : name;
}

function closeDropdowns(event: MouseEvent) {
  if (
    !(
      event.target instanceof Element && event.target.closest("[data-dropdown]")
    )
  ) {
    openDropdown = null;
  }
}
</script>

<svelte:window onclick={closeDropdowns} />

<div class="px-5 py-4">
	<h3 class="mb-3 text-sm font-medium text-zinc-300">Properties</h3>
	<div class="space-y-0">
		<IssueCoreProperties
			{issue}
			{members}
			{states}
			{openDropdown}
			onToggleDropdown={toggleDropdown}
			onCloseDropdown={() => {
				openDropdown = null;
			}}
			{onUpdateIssue}
		/>
		<IssueScheduleProperties {issue} {onUpdateIssue} />
		<IssueLabelProperty
			{issue}
			{labels}
			{openDropdown}
			onToggleDropdown={toggleDropdown}
			{onToggleLabel}
			{onCreateLabel}
		/>
	</div>
</div>
