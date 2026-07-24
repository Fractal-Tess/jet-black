<script lang="ts">
import type { HTMLTextareaAttributes } from "svelte/elements";
import { cn } from "../../utils";

let {
  class: className,
  value = $bindable(),
  ...rest
}: HTMLTextareaAttributes = $props();
</script>

<textarea
  bind:value
  class={cn(
    "flex min-h-32 w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-xs leading-relaxed text-foreground outline-none transition-shadow placeholder:text-muted-foreground/70 focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/20 disabled:cursor-not-allowed disabled:opacity-50",
    className
  )}
  data-slot="textarea"
  {...rest}
></textarea>
