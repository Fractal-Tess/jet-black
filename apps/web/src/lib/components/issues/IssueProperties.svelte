<script lang="ts">
import type { EstimateSystem } from "$lib/estimates";
import IssueCoreProperties from "./IssueCoreProperties.svelte";
import IssueLabelProperty from "./IssueLabelProperty.svelte";
import IssueScheduleProperties from "./IssueScheduleProperties.svelte";
import type {
  CreateLabelInput,
  Issue,
  IssueLabel,
  IssueState,
  ProjectModuleRecord,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  issue,
  labels,
  members,
  modules = [],
  states,
  estimateSystem,
  onCreateLabel,
  onCreateModuleForIssue,
  onToggleLabel,
  onUpdateIssue,
}: {
  issue: Issue;
  labels: IssueLabel[];
  members: WorkspaceMember[];
  modules?: ProjectModuleRecord[];
  states: IssueState[];
  estimateSystem?: EstimateSystem;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateModuleForIssue: (name: string) => Promise<void>;
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
  <h3 class="mb-3 text-sm font-medium text-foreground">Properties</h3>
	<div class="space-y-0">
		<IssueCoreProperties
			{issue}
			{members}
			{modules}
			{states}
			{openDropdown}
			onToggleDropdown={toggleDropdown}
			onCloseDropdown={() => {
				openDropdown = null;
			}}
			{onUpdateIssue}
			{onCreateModuleForIssue}
		/>
		<IssueScheduleProperties {estimateSystem} {issue} {onUpdateIssue} />
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
