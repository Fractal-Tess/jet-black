<script lang="ts">
import type { Id } from "@workspace/convex/dataModel";
import { Badge } from "@workspace/ui/components/badge";
import { Label } from "@workspace/ui/components/label";
import {
  Content as SelectContent,
  Item as SelectItem,
  Root as SelectRoot,
  Trigger as SelectTrigger,
} from "@workspace/ui/components/select";
import type { Issue, Sprint } from "$lib/components/issues/types";

let {
  sprint,
  issues,
  onAssignIssue,
}: {
  sprint: Sprint;
  issues: Issue[];
  onAssignIssue: (
    issueId: Issue["_id"],
    sprintId: Sprint["_id"]
  ) => Promise<void>;
} = $props();

const unassignedIssues = $derived(issues.filter((issue) => !issue.sprintId));

function sprintIssues(sprintId: Id<"sprints">) {
  return issues.filter((issue) => issue.sprintId === sprintId);
}

function completedCount(sprintIssues: Issue[]) {
  return sprintIssues.filter(
    (issue) =>
      issue.state?.type === "completed" || issue.state?.type === "cancelled"
  ).length;
}

function progressPercent(sprintIssues: Issue[]) {
  if (sprintIssues.length === 0) {
    return 0;
  }

  return Math.round((completedCount(sprintIssues) / sprintIssues.length) * 100);
}

function sprintStatus(s: Sprint) {
  const today = new Date().toISOString().slice(0, 10);

  if (s.endDate && s.endDate < today) {
    return "completed";
  }

  if (s.startDate && s.startDate > today) {
    return "upcoming";
  }

  if (s.startDate || s.endDate) {
    return "active";
  }

  return "draft";
}

let selectedIssueId = $state<string | undefined>(undefined);

async function handleAssign(value: string | undefined) {
  if (!value) {
    return;
  }
  await onAssignIssue(value as Issue["_id"], sprint._id);
  selectedIssueId = undefined;
}
</script>

{#key sprint._id}
  {@const assignedIssues = sprintIssues(sprint._id)}
  {@const percent = progressPercent(assignedIssues)}
  <article class="p-4">
    <div
      class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between"
    >
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-2">
          <Badge variant="secondary">
            {sprintStatus(sprint)}
          </Badge>
      <span class="font-mono text-meta text-muted-foreground">
            {sprint.startDate ?? "no start"} \u2192 {sprint.endDate ??
              "no end"}
          </span>
        </div>

        <h3 class="mt-2 text-sm font-semibold text-foreground">
          {sprint.name}
        </h3>
        {#if sprint.description}
          <p class="mt-1 text-sm text-muted-foreground">
            {sprint.description}
          </p>
        {/if}

        <div class="mt-4">
          <div class="mb-1 flex items-center justify-between text-xs">
            <span class="text-muted-foreground">
              {completedCount(assignedIssues)} / {assignedIssues.length}
              tickets done
            </span>
            <span class="font-mono text-secondary-foreground">{percent}%</span>
          </div>
          <div class="h-1.5 overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-primary"
              style:width={`${percent}%`}
            ></div>
          </div>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          {#each assignedIssues as issue (issue._id)}
            <Badge variant="outline">
              <span class="font-mono text-muted-foreground">
                {issue.identifier}
              </span>
              {issue.title}
            </Badge>
          {:else}
            <span class="text-xs text-muted-foreground">
              No tickets in this sprint yet.
            </span>
          {/each}
        </div>
      </div>

      <div class="block w-full shrink-0 lg:w-64">
        <Label class="mb-1 text-xs">Add ticket to sprint</Label>
        <SelectRoot
          type="single"
          value={selectedIssueId}
          onValueChange={handleAssign}
        >
          <SelectTrigger class="w-full">
            {selectedIssueId ? "Selected" : "Select a ticket\u2026"}
          </SelectTrigger>
          <SelectContent>
            {#each unassignedIssues as issue (issue._id)}
              <SelectItem value={issue._id}>
                {issue.identifier} \u00b7 {issue.title}
              </SelectItem>
            {/each}
          </SelectContent>
        </SelectRoot>
      </div>
    </div>
  </article>
{/key}
