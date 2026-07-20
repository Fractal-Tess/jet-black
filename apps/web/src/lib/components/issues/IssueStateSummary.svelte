<script lang="ts">
import { Card } from "@workspace/ui/components/card";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type { Issue, IssueState } from "./types";

let {
  issues,
  states,
}: {
  issues: Issue[];
  states: IssueState[];
} = $props();

function countForState(stateId: string) {
  return issues.filter((issue) => issue.stateId === stateId).length;
}
</script>

<section class="grid gap-3 md:grid-cols-4">
  {#each states as state (state._id)}
    <Card class="p-4">
      <div class="flex items-center justify-between">
        <span
          class="inline-flex items-center gap-2 text-xs font-medium text-muted-foreground"
        >
          <StateTypeIcon class="size-3.5" type={state.type} />
          {state.name}
        </span>
        <span class="font-mono text-xs text-muted-foreground">{state.type}</span>
      </div>
      <p class="mt-5 text-3xl font-semibold tracking-tight text-card-foreground">
        {countForState(state._id)}
      </p>
    </Card>
  {/each}
</section>
