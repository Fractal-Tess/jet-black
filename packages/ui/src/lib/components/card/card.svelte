<script lang="ts">
import type { HTMLAttributes } from "svelte/elements";
import { cn } from "../../utils.js";

let {
  class: className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement> = $props();
</script>

<div class={cn("rounded-2xl border bg-card text-card-foreground", className)} {...props}>{@render children?.()}</div>
