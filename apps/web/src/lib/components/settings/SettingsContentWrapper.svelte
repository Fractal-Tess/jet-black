<script lang="ts">
import type { Snippet } from "svelte";

let {
  children,
  hugging = false,
}: {
  children: Snippet;
  hugging?: boolean;
} = $props();
</script>

<div class="flex size-full grow flex-col overflow-hidden">
  <div class="flex-1 overflow-y-auto">
    <div
      class="py-8 {hugging
        ? 'w-full px-6 lg:px-10'
        : 'mx-auto w-full max-w-[56.25rem] px-6 md:px-8'}"
    >
      {@render children()}
    </div>
  </div>
</div>
