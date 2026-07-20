<script lang="ts">
import Ban from "lucide-svelte/icons/ban";
import CircleAlert from "lucide-svelte/icons/circle-alert";
import SignalHigh from "lucide-svelte/icons/signal-high";
import SignalLow from "lucide-svelte/icons/signal-low";
import SignalMedium from "lucide-svelte/icons/signal-medium";
import type { IssuePriority } from "./types";

let {
  priority,
  class: className = "size-3.5",
}: {
  priority: IssuePriority;
  class?: string;
} = $props();

const colorClass: Record<IssuePriority, string> = {
  high: "text-warning",
  low: "text-info",
  medium: "text-primary",
  none: "text-muted-foreground",
  urgent: "text-destructive",
};
</script>

{#if priority === "urgent"}
  <CircleAlert class="{className} shrink-0 {colorClass.urgent}" />
{:else if priority === "high"}
  <SignalHigh class="{className} shrink-0 {colorClass.high}" />
{:else if priority === "medium"}
  <SignalMedium class="{className} shrink-0 {colorClass.medium}" />
{:else if priority === "low"}
  <SignalLow class="{className} shrink-0 {colorClass.low}" />
{:else}
  <Ban class="{className} shrink-0 {colorClass.none}" />
{/if}
