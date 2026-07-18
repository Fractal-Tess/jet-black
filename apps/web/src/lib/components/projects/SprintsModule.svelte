<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Card } from "@workspace/ui/components/card";
import type { Issue, Sprint } from "$lib/components/issues/types";
import SprintCard from "./SprintCard.svelte";
import SprintForm from "./SprintForm.svelte";

let {
  creating,
  issues,
  onAssignIssue,
  onCreate,
  sprints,
}: {
  creating: boolean;
  issues: Issue[];
  onAssignIssue: (
    issueId: Issue["_id"],
    sprintId: Sprint["_id"]
  ) => Promise<void>;
  onCreate: (input: {
    description?: string;
    endDate?: string;
    name: string;
    startDate?: string;
  }) => Promise<void>;
  sprints: Sprint[];
} = $props();

const unassignedIssues = $derived(issues.filter((issue) => !issue.sprintId));
</script>

<section class="grid gap-5 xl:grid-cols-[360px_minmax(0,1fr)]">
  <div class="space-y-5">
    <SprintForm {creating} {onCreate} />

    <Card class="p-4">
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
        Scope pool
      </p>
      <p class="mt-2 text-2xl font-semibold text-foreground">
        {unassignedIssues.length}
      </p>
      <p class="text-xs text-muted-foreground">tickets not assigned to a sprint</p>
    </Card>
  </div>

  <Card>
    <div
      class="flex items-center justify-between border-b border-border px-4 py-3"
    >
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
          Timeline
        </p>
        <h2 class="mt-1 text-lg font-semibold text-foreground">
          Project sprints
        </h2>
      </div>
      <Badge variant="outline">
        {sprints.length} total
      </Badge>
    </div>

    <div class="divide-y divide-border">
      {#each sprints as sprint (sprint._id)}
        <SprintCard {sprint} {issues} {onAssignIssue} />
      {:else}
        <div class="grid min-h-80 place-items-center p-8 text-center">
          <div>
            <p class="text-sm font-medium text-secondary-foreground">
              No sprints yet.
            </p>
            <p class="mt-1 max-w-sm text-sm text-muted-foreground">
              Create the first sprint to time-box tickets and track delivery
              progress.
            </p>
          </div>
        </div>
      {/each}
    </div>
  </Card>
</section>
