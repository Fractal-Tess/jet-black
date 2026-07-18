<script lang="ts">
import CalendarDays from "lucide-svelte/icons/calendar-days";
import Clock from "lucide-svelte/icons/clock";
import Target from "lucide-svelte/icons/target";
import DatePicker from "./DatePicker.svelte";
import type { Issue, UpdateIssueInput } from "./types";

let {
  issue,
  onUpdateIssue,
}: {
  issue: Issue;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
} = $props();

function handleStartDateChange(date: string) {
  onUpdateIssue({ startDate: date || null });
}

function handleTargetDateChange(date: string) {
  onUpdateIssue({ targetDate: date || null });
}

function handleEstimateChange(value: string) {
  onUpdateIssue({ estimate: value ? Number(value) : null });
}
</script>

<!-- Start date -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-zinc-500">
		<CalendarDays class="size-4" />
		<span>Start date</span>
	</div>
	<DatePicker
		value={issue.startDate}
		placeholder="Add start date"
		maxDate={issue.targetDate}
		onChange={(date) => handleStartDateChange(date ?? "")}
	/>
</div>

<!-- Due date -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-zinc-500">
		<Clock class="size-4" />
		<span>Due date</span>
	</div>
	<DatePicker
		value={issue.targetDate}
		placeholder="Add due date"
		minDate={issue.startDate}
		onChange={(date) => handleTargetDateChange(date ?? "")}
	/>
</div>

<!-- Estimate -->
<div class="flex items-center gap-3 py-2.5">
	<div class="flex w-[140px] shrink-0 items-center gap-2 text-sm text-zinc-500">
		<Target class="size-4" />
		<span>Estimate</span>
	</div>
	<input
		type="number"
		min="0"
		value={issue.estimate ?? ""}
		placeholder="No estimate"
		onchange={(e) => handleEstimateChange(e.currentTarget.value)}
		class="h-7 w-24 rounded-md border border-transparent bg-transparent px-1.5 text-sm text-zinc-300 outline-none transition placeholder:text-zinc-500 hover:border-white/10 hover:bg-white/[0.03] focus:border-amber-400/60"
	/>
</div>
