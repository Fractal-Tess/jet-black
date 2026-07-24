<script lang="ts">
import type { HTMLInputAttributes } from "svelte/elements";
import { cn } from "../../utils";

let {
  class: className,
  value = $bindable(),
  ...rest
}: HTMLInputAttributes = $props();
</script>

<input
  bind:value
  class={cn(
    "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-xs text-foreground outline-none transition-shadow placeholder:text-muted-foreground/70 focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/20 disabled:cursor-not-allowed disabled:opacity-50",
    className
  )}
  data-slot="input"
  {...rest}
/>
