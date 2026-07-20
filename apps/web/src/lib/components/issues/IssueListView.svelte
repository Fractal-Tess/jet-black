<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import { Input } from "@workspace/ui/components/input";
import ChevronRight from "lucide-svelte/icons/chevron-right";
import Plus from "lucide-svelte/icons/plus";
import {
  groupIssues,
  type IssueDisplayOptions,
  type IssueGroup,
} from "./display-options";
import IssuePropertyChips from "./IssuePropertyChips.svelte";
import PriorityIcon from "./PriorityIcon.svelte";
import StateTypeIcon from "./StateTypeIcon.svelte";
import type {
  Issue,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  displayOptions,
  issues,
  members,
  onQuickCreate,
  onSelect,
  onUpdateIssueFor,
  selectedIssueId,
  states,
}: {
  displayOptions: IssueDisplayOptions;
  issues: Issue[];
  members: WorkspaceMember[];
  onQuickCreate: (state: IssueState, title: string) => Promise<void>;
  onSelect: (issue: Issue) => void;
  onUpdateIssueFor: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  selectedIssueId?: string;
  states: IssueState[];
} = $props();

let collapsedGroupIds = $state<string[]>([]);
let creatingGroupId = $state<string | null>(null);
let quickTitles = $state<Record<string, string>>({});

const groups = $derived(groupIssues(issues, states, displayOptions));

function isCollapsed(groupId: string) {
  return collapsedGroupIds.includes(groupId);
}

function toggleGroup(groupId: string) {
  collapsedGroupIds = isCollapsed(groupId)
    ? collapsedGroupIds.filter((id) => id !== groupId)
    : [...collapsedGroupIds, groupId];
}

async function submitQuickCreate(group: IssueGroup) {
  const title = (quickTitles[group.id] ?? "").trim();

  if (!(title && group.state)) {
    return;
  }

  creatingGroupId = group.id;

  try {
    await onQuickCreate(group.state, title);
    quickTitles = { ...quickTitles, [group.id]: "" };
  } finally {
    creatingGroupId = null;
  }
}
</script>

<div class="flex flex-col">
  {#each groups as group (group.id)}
    <section aria-label={`${group.name} group`} class="border-b border-border">
      <div class="flex items-center gap-2 px-3 py-2.5">
        <Button
          aria-expanded={!isCollapsed(group.id)}
          aria-label={`Toggle ${group.name} group`}
          class="size-5"
          onclick={() => toggleGroup(group.id)}
          size="icon-xs"
          variant="ghost"
        >
          <ChevronRight
            class="size-3.5 transition-transform {isCollapsed(group.id)
              ? ''
              : 'rotate-90'}"
          />
        </Button>
        {#if group.state}
          <StateTypeIcon
            class="size-3.5"
            type={group.state.type}
          />
        {:else if group.priority}
          <PriorityIcon class="size-3.5" priority={group.priority} />
        {/if}
        <h3 class="text-sm font-medium text-foreground">{group.name}</h3>
        <span class="text-sm tabular-nums text-muted-foreground">
          {group.issues.length}
        </span>
        {#if group.state}
          <Button
            aria-label={`New work item in ${group.name}`}
            class="ml-1 size-5"
            onclick={() =>
              document.getElementById(`list-quick-add-${group.id}`)?.focus()}
            size="icon-xs"
            variant="ghost"
          >
            <Plus class="size-3.5" />
          </Button>
        {/if}
      </div>

      {#if !isCollapsed(group.id)}
        <div>
          {#each group.issues as issue (issue._id)}
            <div
              class="flex min-h-11 items-center gap-3 border-t border-border py-2 pr-3 pl-10 transition-colors hover:bg-muted/50 {selectedIssueId ===
              issue._id
                ? 'bg-accent/50'
                : ''}"
            >
              {#if displayOptions.properties.key}
                <span
                   class="w-16 shrink-0 font-mono text-meta text-muted-foreground"
                >
                  {issue.identifier}
                </span>
              {/if}
              <button
                class="min-w-0 flex-1 cursor-pointer truncate text-left text-sm font-medium text-foreground transition-colors hover:text-primary"
                onclick={() => onSelect(issue)}
                type="button"
              >
                {issue.title}
              </button>
              <div class="hidden shrink-0 sm:block">
                <IssuePropertyChips
                  {issue}
                  {members}
                  onUpdate={(input) => onUpdateIssueFor(issue, input)}
                  properties={displayOptions.properties}
                />
              </div>
            </div>
          {/each}

          {#if group.state}
            <form
              class="flex items-center gap-2 border-t border-border py-2 pr-3 pl-10 transition-colors focus-within:bg-muted/30"
              onsubmit={(event) => {
                event.preventDefault();
                submitQuickCreate(group);
              }}
            >
              <Plus class="size-3.5 shrink-0 text-muted-foreground" />
              <Input
                aria-label={`Quick issue title for ${group.name}`}
                class="h-6 min-w-0 flex-1 border-0 bg-transparent px-0 text-sm shadow-none focus-visible:ring-0"
                id={`list-quick-add-${group.id}`}
                oninput={(event) =>
                  (quickTitles = {
                    ...quickTitles,
                    [group.id]: event.currentTarget.value,
                  })}
                placeholder="New work item"
                value={quickTitles[group.id] ?? ""}
              />
              {#if (quickTitles[group.id] ?? "").trim()}
                <Button
                  aria-label={`Add issue to ${group.name}`}
                  disabled={creatingGroupId === group.id}
                  size="xs"
                  type="submit"
                >
                  {creatingGroupId === group.id ? "Adding…" : "Add"}
                </Button>
              {/if}
            </form>
          {/if}
        </div>
      {/if}
    </section>
  {/each}
</div>
