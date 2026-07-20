<script lang="ts">
import { formatFullDate, formatRelativeTime } from "./relative-time";
import type { IssueActivity } from "./types";

let { activity }: { activity: IssueActivity } = $props();
</script>

<div class="flex items-start gap-3">
  <span
    class="grid size-7 shrink-0 place-items-center rounded-full border border-border bg-background"
  >
    <span class="size-1.5 rounded-full bg-muted-foreground"></span>
  </span>
  <p class="min-w-0 flex-1 pt-1.5 text-xs text-muted-foreground">
    <span class="font-medium text-foreground">{activity.actor.name}</span>
    {activity.message}
    <span
      class="whitespace-nowrap text-muted-foreground/70"
      title={formatFullDate(activity._creationTime)}
    >
      · {formatRelativeTime(activity._creationTime)}
    </span>
  </p>
</div>
