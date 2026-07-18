<script lang="ts">
import type { Snippet } from "svelte";

let {
  label,
  children,
}: {
  label: string;
  children: Snippet;
} = $props();
</script>

<div class="mt-4 first:mt-2">
  <p
    class="mb-1 px-2 text-[11px] font-semibold uppercase tracking-wider text-sidebar-muted-foreground"
  >
    {label}
  </p>
  <div class="flex flex-col gap-0.5">
    {@render children()}
  </div>
</div>
