<script lang="ts">
import type { Snippet } from "svelte";

let {
  header,
  children,
}: {
  header: Snippet;
  children: Snippet;
} = $props();
</script>

<div
  class="flex h-full w-[250px] shrink-0 flex-col overflow-y-auto border-r border-sidebar-border bg-sidebar"
>
  <div class="px-3 pt-4 pb-2">
    {@render header()}
  </div>
  <nav class="flex flex-1 flex-col gap-0.5 px-3 pb-4">
    {@render children()}
  </nav>
</div>
