<script lang="ts">
import type { Snippet } from "svelte";

let {
  title,
  description,
  control,
  danger = false,
}: {
  title: string;
  description?: string;
  control?: Snippet;
  danger?: boolean;
} = $props();
</script>

<div
  class="flex w-full flex-col items-start gap-3 rounded-lg border px-4 py-3 md:flex-row md:items-center md:justify-between md:gap-8
    {danger
    ? 'border-destructive/40 bg-destructive/10'
    : 'border-border bg-card'}"
>
  <div class="flex flex-col gap-1">
    <h4
      class="text-sm font-medium {danger
        ? 'text-destructive'
        : 'text-foreground'}"
    >
      {title}
    </h4>
    {#if description}
      <p
        class="text-xs {danger
          ? 'text-destructive/60'
          : 'text-muted-foreground'}"
      >
        {description}
      </p>
    {/if}
  </div>
  {#if control}
    <div class="shrink-0">
      {@render control()}
    </div>
  {/if}
</div>
