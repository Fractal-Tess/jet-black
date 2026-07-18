<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Card } from "@workspace/ui/components/card";
import type { Issue, ProjectModuleRecord } from "$lib/components/issues/types";
import ModuleCard from "./ModuleCard.svelte";
import ModuleForm from "./ModuleForm.svelte";

let {
  creating,
  issues,
  modules,
  onAssignIssue,
  onCreate,
}: {
  creating: boolean;
  issues: Issue[];
  modules: ProjectModuleRecord[];
  onAssignIssue: (
    issueId: Issue["_id"],
    moduleId: ProjectModuleRecord["_id"]
  ) => Promise<void>;
  onCreate: (input: {
    description?: string;
    name: string;
    status?: ProjectModuleRecord["status"];
    targetDate?: string;
  }) => Promise<void>;
} = $props();

const unassignedIssues = $derived(issues.filter((issue) => !issue.moduleId));
</script>

<section class="grid gap-5 xl:grid-cols-[360px_minmax(0,1fr)]">
  <div class="space-y-5">
    <ModuleForm {creating} {onCreate} />

    <Card class="p-4">
      <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
        Unsorted
      </p>
      <p class="mt-2 text-2xl font-semibold text-foreground">
        {unassignedIssues.length}
      </p>
      <p class="text-xs text-muted-foreground">tickets not assigned to a module</p>
    </Card>
  </div>

  <Card>
    <div
      class="flex items-center justify-between border-b border-border px-4 py-3"
    >
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
          Map
        </p>
        <h2 class="mt-1 text-lg font-semibold text-foreground">
          Project modules
        </h2>
      </div>
      <Badge variant="outline">
        {modules.length} total
      </Badge>
    </div>

    <div class="grid gap-3 p-4 lg:grid-cols-2">
      {#each modules as projectModule (projectModule._id)}
        <ModuleCard module={projectModule} {issues} {onAssignIssue} />
      {:else}
        <div
          class="col-span-full grid min-h-80 place-items-center p-8 text-center"
        >
          <div>
            <p class="text-sm font-medium text-secondary-foreground">
              No modules yet.
            </p>
            <p class="mt-1 max-w-sm text-sm text-muted-foreground">
              Create modules to group related tickets by product area or
              milestone.
            </p>
          </div>
        </div>
      {/each}
    </div>
  </Card>
</section>
