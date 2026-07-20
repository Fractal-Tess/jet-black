<script lang="ts">
import type { HTMLInputAttributes } from "svelte/elements";
import { cn, type WithElementRef } from "../../utils.js";

let {
  ref = $bindable(null),
  class: className,
  value = $bindable(),
  ...props
}: WithElementRef<HTMLInputAttributes, HTMLInputElement> = $props();
</script>

<input
	bind:this={ref}
	bind:value
	data-slot="input"
	class={cn(
		"h-8 w-full rounded-lg border border-input bg-background px-2.5 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20",
		className
	)}
	{...props}
/>
