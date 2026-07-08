<script lang="ts">
import type { HTMLInputAttributes } from "svelte/elements";
import { cn } from "../../utils.js";

let {
  class: className,
  value = $bindable(),
  ...props
}: HTMLInputAttributes = $props();
</script>

<input bind:value class={cn("h-11 w-full rounded-xl border bg-background px-3 outline-none transition focus:border-primary focus:ring-2 focus:ring-primary/20", className)} {...props} />
