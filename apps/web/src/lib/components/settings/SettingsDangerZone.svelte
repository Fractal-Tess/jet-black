<script lang="ts">
import type { Snippet } from "svelte";

let {
  children,
}: {
  children: Snippet;
} = $props();
</script>

<div class="mt-10">
  <h4 class="mb-3 text-sm font-medium text-destructive">Danger zone</h4>
  <div class="flex flex-col gap-3">
    {@render children()}
  </div>
</div>
