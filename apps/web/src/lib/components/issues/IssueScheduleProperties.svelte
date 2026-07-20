<script lang="ts">
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from "@workspace/ui/components/select";
import CalendarDays from "lucide-svelte/icons/calendar-days";
import ChevronDown from "lucide-svelte/icons/chevron-down";
import Clock from "lucide-svelte/icons/clock";
import Target from "lucide-svelte/icons/target";
import type { EstimateSystem } from "$lib/estimates";
import DatePicker from "./DatePicker.svelte";
import type { Issue, UpdateIssueInput } from "./types";

let {
  issue,
  estimateSystem,
  onUpdateIssue,
}: {
  issue: Issue;
  estimateSystem?: EstimateSystem;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
} = $props();

const NO_ESTIMATE = "none";
const FALLBACK_ESTIMATE_VALUES = [1, 2, 3, 5, 8, 13];

const showEstimate = $derived(
  estimateSystem === undefined || estimateSystem.enabled
);

const storyPointOptions = $derived([
  { label: "No estimate", value: NO_ESTIMATE },
  ...(estimateSystem?.enabled
    ? estimateSystem.values
    : FALLBACK_ESTIMATE_VALUES
  ).map((point) => ({
    label: `${point} ${point === 1 ? "point" : "points"}`,
    value: String(point),
  })),
]);

const estimateValue = $derived(
  issue.estimate === undefined ? NO_ESTIMATE : String(issue.estimate)
);
const estimateLabel = $derived(
  storyPointOptions.find((option) => option.value === estimateValue)?.label ??
    `${issue.estimate} points`
);

function handleStartDateChange(date: string) {
  onUpdateIssue({ startDate: date || null });
}

function handleTargetDateChange(date: string) {
  onUpdateIssue({ targetDate: date || null });
}

function handleEstimateChange(value: string) {
  onUpdateIssue({ estimate: value === NO_ESTIMATE ? null : Number(value) });
}
</script>

<!-- Start date -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
		<CalendarDays class="size-4" />
		<span>Start date</span>
	</div>
	<DatePicker
		value={issue.startDate}
		placeholder="Add start date"
		maxDate={issue.targetDate}
		disabledHint="The start date must be before the due date"
		onChange={(date) => handleStartDateChange(date ?? "")}
	/>
</div>

<!-- Due date -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
		<Clock class="size-4" />
		<span>Due date</span>
	</div>
	<DatePicker
		value={issue.targetDate}
		placeholder="Add due date"
		minDate={issue.startDate}
		disabledHint="The due date must be after the start date"
		onChange={(date) => handleTargetDateChange(date ?? "")}
	/>
</div>

<!-- Estimate (story points) -->
{#if showEstimate}
	<div class="flex items-center gap-3 py-2.5">
		<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-muted-foreground">
			<Target class="size-4" />
			<span>Estimate</span>
		</div>
		<Select
			onValueChange={(next) => {
				if (next) {
					handleEstimateChange(next);
				}
			}}
			type="single"
			value={estimateValue}
		>
			<SelectTrigger
				aria-label="Estimate"
				class="flex h-7 items-center gap-2 rounded-lg border-0 bg-transparent px-2 py-1 text-sm shadow-none transition-colors hover:bg-muted {estimateValue === NO_ESTIMATE ? 'text-muted-foreground' : 'text-foreground'}"
			>
				{estimateLabel}
				<ChevronDown class="size-3 text-muted-foreground" />
			</SelectTrigger>
			<SelectContent class="border border-border bg-popover text-popover-foreground">
				{#each storyPointOptions as option (option.value)}
					<SelectItem class="text-sm" label={option.label} value={option.value} />
				{/each}
			</SelectContent>
		</Select>
	</div>
{/if}
