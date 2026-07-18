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
import type { Issue, ProjectModuleRecord } from "$lib/components/issues/types";

let {
  module: projectModule,
  issues,
  onAssignIssue,
}: {
  module: ProjectModuleRecord;
  issues: Issue[];
  onAssignIssue: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
} = $props();

const unassignedIssues = $derived(issues.filter((issue) => !issue.moduleId));

function moduleIssues(moduleId: Id<"projectModules">) {
  return issues.filter((issue) => issue.moduleId === moduleId);
}

function completedCount(moduleIssues: Issue[]) {
  return moduleIssues.filter(
    (issue) =>
      issue.state?.type === "completed" || issue.state?.type === "cancelled"
  ).length;
}

function progressPercent(moduleIssues: Issue[]) {
  if (moduleIssues.length === 0) {
    return 0;
  }

  return Math.round((completedCount(moduleIssues) / moduleIssues.length) * 100);
}

function statusLabel(moduleStatus: ProjectModuleRecord["status"]) {
  return moduleStatus.replace("_", " ");
}

let selectedIssueId = $state<string | undefined>(undefined);

async function handleAssign(value: string | undefined) {
  if (!value) {
    return;
  }
  await onAssignIssue(value as Issue["_id"], projectModule._id);
  selectedIssueId = undefined;
}

const assignedIssues = $derived(moduleIssues(projectModule._id));
const percent = $derived(progressPercent(assignedIssues));
</script>

<article class="rounded-lg border border-border bg-card p-4">
  <div class="flex items-start justify-between gap-3">
    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-2">
        <Badge variant="secondary" class="capitalize">
          {statusLabel(projectModule.status)}
        </Badge>
        {#if projectModule.targetDate}
          <span class="font-mono text-[10px] text-muted-foreground">
            due {projectModule.targetDate}
          </span>
        {/if}
      </div>
      <h3 class="mt-2 text-sm font-semibold text-foreground">
        {projectModule.name}
      </h3>
    </div>
    <span class="font-mono text-xs text-muted-foreground">{percent}%</span>
  </div>

  {#if projectModule.description}
    <p class="mt-2 text-sm text-muted-foreground">
      {projectModule.description}
    </p>
  {/if}

  <div class="mt-4">
    <div class="mb-1 flex items-center justify-between text-xs">
      <span class="text-muted-foreground">
        {completedCount(assignedIssues)} / {assignedIssues.length}
        tickets done
      </span>
    </div>
    <div class="h-1.5 overflow-hidden rounded-full bg-muted">
      <div
        class="h-full rounded-full bg-primary"
        style:width={`${percent}%`}
      ></div>
    </div>
  </div>

  <div class="mt-4">
    <Label class="mb-1 text-xs">Add ticket to module</Label>
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
        No tickets in this module yet.
      </span>
    {/each}
  </div>
</article>
