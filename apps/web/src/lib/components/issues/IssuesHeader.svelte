<script lang="ts">
import Columns3 from "lucide-svelte/icons/columns-3";
import List from "lucide-svelte/icons/list";
import Plus from "lucide-svelte/icons/plus";
import Search from "lucide-svelte/icons/search";

import type { IssueDisplayOptions } from "$lib/components/issues/display-options";
import IssueDisplayDropdown from "$lib/components/issues/IssueDisplayDropdown.svelte";
import type { Project } from "$lib/components/issues/types";
import {
  projectModuleHref,
  workspaceHref,
  workspaceProjectsHref,
} from "$lib/routes";

let {
  activeProject,
  displayOptions,
  issueCount,
  issueView,
  onAddWorkItem,
  onDisplayOptionsChange,
  onIssueViewChange,
  onSearchChange,
  searchQuery,
  workspaceSlug,
}: {
  activeProject: Project | null;
  displayOptions: IssueDisplayOptions;
  issueCount: number;
  issueView: "board" | "list";
  onAddWorkItem: () => void;
  onDisplayOptionsChange: (options: IssueDisplayOptions) => void;
  onIssueViewChange: (view: "board" | "list") => void;
  onSearchChange: (query: string) => void;
  searchQuery: string;
  workspaceSlug: string;
} = $props();
</script>

<div class="flex flex-col gap-3 border-b border-border px-5 py-3 sm:flex-row sm:items-center sm:justify-between sm:px-8">
  <div class="flex min-w-0 items-center gap-2.5 overflow-hidden">
    <div class="flex min-w-0 items-center gap-1.5 overflow-hidden whitespace-nowrap text-sm text-muted-foreground">
      <a
        href={workspaceHref(workspaceSlug)}
        class="transition-colors hover:text-foreground"
      >
        Home
      </a>
      <span class="text-muted-foreground">/</span>
      <a
        href={workspaceProjectsHref(workspaceSlug)}
        class="transition-colors hover:text-foreground"
      >
        Projects
      </a>
      {#if activeProject}
      <span class="text-muted-foreground">/</span>
        <a
          href={projectModuleHref({
            module: "tickets",
            projectId: activeProject._id,
            workspaceSlug,
          })}
        class="transition-colors hover:text-foreground"
        >
          <span class="block max-w-28 truncate sm:max-w-48">{activeProject.name}</span>
        </a>
      <span class="text-muted-foreground">/</span>
      <span class="font-medium text-foreground">Work Items</span>
      {/if}
    </div>

    {#if issueCount > 0}
      <span
        class="inline-flex items-center rounded-md border border-border px-2 py-0.5 text-meta tabular-nums text-muted-foreground"
      >
        {issueCount}
      </span>
    {/if}
  </div>

  <div class="flex w-full min-w-0 items-center gap-2 sm:w-auto">
    <div class="relative min-w-32 flex-1 sm:w-52 sm:flex-none">
      <Search class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
      <input
        aria-label="Search issues"
        class="h-8 w-full rounded-lg border border-input bg-background pl-7 pr-2 text-xs text-foreground outline-none placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        oninput={(e) => onSearchChange(e.currentTarget.value)}
        placeholder="Search issues…"
        value={searchQuery}
      />
    </div>

    <div
      class="inline-flex items-center gap-1 rounded-lg bg-muted p-1"
      role="group"
      aria-label="Layout"
    >
      <button
        aria-label="List"
        aria-pressed={issueView === "list"}
        class="grid h-[22px] w-7 place-items-center rounded-sm transition {issueView ===
        'list'
          ? 'bg-background text-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => onIssueViewChange("list")}
        title="List layout"
        type="button"
      >
        <List class="size-3.5" />
      </button>
      <button
        aria-label="Board"
        aria-pressed={issueView === "board"}
        class="grid h-[22px] w-7 place-items-center rounded-sm transition {issueView ===
        'board'
          ? 'bg-background text-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => onIssueViewChange("board")}
        title="Board layout"
        type="button"
      >
        <Columns3 class="size-3.5" />
      </button>
    </div>

    <IssueDisplayDropdown {displayOptions} onChange={onDisplayOptionsChange} />

    <button
      aria-label="Add work item"
      class="flex h-8 shrink-0 items-center gap-1.5 rounded-lg bg-primary px-2 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/80 sm:px-3"
      onclick={onAddWorkItem}
      type="button"
    >
      <Plus class="size-3.5" />
      <span class="hidden sm:inline">Add work item</span>
    </button>
  </div>
</div>
