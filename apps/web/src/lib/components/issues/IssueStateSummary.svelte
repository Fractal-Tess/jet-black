<script lang="ts">
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
    <article class="rounded-xl border border-white/10 bg-[#151616] p-4">
      <div class="flex items-center justify-between">
        <span
          class="inline-flex items-center gap-2 text-xs font-medium text-zinc-400"
        >
          <span
            class="size-2 rounded-full"
            style:background-color={state.color}
          ></span>
          {state.name}
        </span>
        <span class="font-mono text-xs text-zinc-600">{state.type}</span>
      </div>
      <p class="mt-5 text-3xl font-semibold tracking-tight text-zinc-100">
        {countForState(state._id)}
      </p>
    </article>
  {/each}
</section>
