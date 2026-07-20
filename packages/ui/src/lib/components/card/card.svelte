<script lang="ts">
import type { HTMLAttributes } from "svelte/elements";
import { cn } from "../../utils.js";

let {
  class: className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement> = $props();
</script>

<div
	data-slot="card"
	class={cn("rounded-xl border border-border bg-card text-card-foreground shadow-sm", className)}
	{...props}
>
	{@render children?.()}
</div>
