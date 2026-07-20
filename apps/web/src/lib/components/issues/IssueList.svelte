<script lang="ts">
import { Badge } from "@workspace/ui/components/badge";
import { Card } from "@workspace/ui/components/card";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type { Issue } from "./types";

let {
  issues,
  selectedIssueId,
  onSelect,
}: {
  issues: Issue[];
  onSelect: (issue: Issue) => void;
  selectedIssueId?: string;
} = $props();

const priorityLabel: Record<Issue["priority"], string> = {
  high: "High",
  low: "Low",
  medium: "Medium",
  none: "None",
  urgent: "Urgent",
};

function formatAge(timestamp: number) {
  const days = Math.max(0, Math.floor((Date.now() - timestamp) / 86_400_000));

  if (days === 0) {
    return "today";
  }

  if (days === 1) {
    return "1 day ago";
  }

  return `${days} days ago`;
}
</script>

<Card>
<section>
  <div
    class="flex items-center justify-between border-b border-border px-4 py-3"
  >
    <div>
      <p class="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        Tickets
      </p>
      <h2 class="mt-1 text-lg font-semibold text-card-foreground">
        Work item inbox
      </h2>
    </div>
    <Badge variant="outline">
      {issues.length} total
    </Badge>
  </div>

  <div class="divide-y divide-border">
    {#each issues as issue (issue._id)}
      <button
        class="grid w-full grid-cols-[auto_1fr] gap-3 px-4 py-3 text-left transition-colors hover:bg-muted/50 sm:grid-cols-[auto_1fr_auto] {selectedIssueId ===
        issue._id
          ? 'bg-accent/50'
          : ''}"
        onclick={() => onSelect(issue)}
        type="button"
      >
        <span
          class="mt-0.5 grid size-8 place-items-center rounded-lg bg-muted font-mono text-meta text-muted-foreground"
        >
          {issue.identifier.split("-").at(-1)}
        </span>

        <span class="min-w-0">
          <span class="flex flex-wrap items-center gap-2">
            <span class="font-mono text-meta text-muted-foreground">
              {issue.identifier}
            </span>
            <Badge class="gap-1" variant="outline">
              {#if issue.state}
                <StateTypeIcon class="size-3" type={issue.state.type} />
              {/if}
              {issue.state?.name ?? "No state"}
            </Badge>
            {#if issue.priority !== "none"}
              <Badge class="gap-1" variant="secondary">
                <PriorityIcon class="size-3" priority={issue.priority} />
                {priorityLabel[issue.priority]}
              </Badge>
            {/if}
          </span>
          <span class="mt-1 block truncate text-sm text-foreground">
            {issue.title}
          </span>
          {#if issue.description}
            <span class="mt-1 block truncate text-xs text-muted-foreground">
              {issue.description}
            </span>
          {/if}
        </span>

        <span class="hidden self-center text-xs text-muted-foreground sm:block">
          {formatAge(issue.updatedAt)}
        </span>
      </button>
    {:else}
      <div class="px-4 py-12 text-center">
        <div
          class="mx-auto grid size-12 place-items-center rounded-xl border border-border bg-muted text-xl text-muted-foreground"
        >
          ◇
        </div>
        <p class="mt-4 text-sm text-muted-foreground">No tickets yet.</p>
        <p class="mt-1 text-xs text-muted-foreground">
          Create the first work item to start the project loop.
        </p>
      </div>
    {/each}
  </div>
</section>
</Card>
