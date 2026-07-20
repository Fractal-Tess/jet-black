<script lang="ts">
import type { HTMLTextareaAttributes } from "svelte/elements";
import { cn, type WithElementRef, type WithoutChildren } from "../../utils.js";

let {
  ref = $bindable(null),
  value = $bindable(),
  class: className,
  "data-slot": dataSlot = "textarea",
  ...restProps
}: WithoutChildren<WithElementRef<HTMLTextareaAttributes>> = $props();
</script>

<textarea
	bind:this={ref}
	data-slot={dataSlot}
	class={cn(
		"flex min-h-16 w-full field-sizing-content rounded-lg border border-input bg-background px-2.5 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20",
		className
	)}
	bind:value
	{...restProps}
></textarea>
