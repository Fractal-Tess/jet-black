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
  class="flex w-full flex-col items-start gap-3 py-3 md:flex-row md:items-center md:justify-between md:gap-8"
>
  <div class="flex flex-col gap-0.5">
    <h4 class="text-sm font-medium text-foreground">{title}</h4>
    {#if description}
      <p class="text-xs text-muted-foreground">{description}</p>
    {/if}
  </div>
  {#if control}
    <div class="shrink-0">
      {@render control()}
    </div>
  {/if}
</div>
