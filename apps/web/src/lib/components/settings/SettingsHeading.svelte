<script lang="ts">
import type { Snippet } from "svelte";

let {
  title,
  description,
  control,
}: {
  title: string;
  description?: string;
  control?: Snippet;
} = $props();
</script>

<div
  class="flex flex-col items-start justify-between gap-3 md:flex-row md:items-center"
>
  <div class="flex flex-col gap-1">
    <h3 class="text-lg font-medium text-foreground">{title}</h3>
    {#if description}
      <p class="text-sm text-muted-foreground">{description}</p>
    {/if}
  </div>
  {#if control}
    <div class="shrink-0">
      {@render control()}
    </div>
  {/if}
</div>
