<script lang="ts">
import Columns3 from "lucide-svelte/icons/columns-3";
import List from "lucide-svelte/icons/list";
import Plus from "lucide-svelte/icons/plus";

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
  workspaceSlug,
}: {
  activeProject: Project | null;
  displayOptions: IssueDisplayOptions;
  issueCount: number;
  issueView: "board" | "list";
  onAddWorkItem: () => void;
  onDisplayOptionsChange: (options: IssueDisplayOptions) => void;
  onIssueViewChange: (view: "board" | "list") => void;
  workspaceSlug: string;
} = $props();
</script>

<div class="flex items-center justify-between border-b border-white/10 px-5 py-3 sm:px-8">
  <div class="flex min-w-0 items-center gap-2.5">
    <div class="flex items-center gap-1.5 text-sm text-zinc-400">
      <a
        href={workspaceHref(workspaceSlug)}
        class="transition hover:text-zinc-200"
      >
        Home
      </a>
      <span class="text-zinc-700">/</span>
      <a
        href={workspaceProjectsHref(workspaceSlug)}
        class="transition hover:text-zinc-200"
      >
        Projects
      </a>
      {#if activeProject}
        <span class="text-zinc-700">/</span>
        <a
          href={projectModuleHref({
            module: "tickets",
            projectId: activeProject._id,
            workspaceSlug,
          })}
          class="transition hover:text-zinc-200"
        >
          {activeProject.name}
        </a>
        <span class="text-zinc-700">/</span>
        <span class="font-medium text-zinc-100">Work Items</span>
      {/if}
    </div>

    {#if issueCount > 0}
      <span
        class="inline-flex items-center rounded-md border border-white/10 px-2 py-0.5 text-[11px] tabular-nums text-zinc-500"
      >
        {issueCount}
      </span>
    {/if}
  </div>

  <div class="flex items-center gap-2">
    <div
      class="inline-flex items-center gap-1 rounded-md bg-white/[0.06] p-1"
      role="group"
      aria-label="Layout"
    >
      <button
        aria-label="List"
        aria-pressed={issueView === "list"}
        class="grid h-[22px] w-7 place-items-center rounded-sm transition {issueView ===
        'list'
          ? 'bg-[#101111] text-zinc-100 shadow-sm'
          : 'text-zinc-500 hover:text-zinc-200'}"
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
          ? 'bg-[#101111] text-zinc-100 shadow-sm'
          : 'text-zinc-500 hover:text-zinc-200'}"
        onclick={() => onIssueViewChange("board")}
        title="Board layout"
        type="button"
      >
        <Columns3 class="size-3.5" />
      </button>
    </div>

    <IssueDisplayDropdown {displayOptions} onChange={onDisplayOptionsChange} />

    <button
      class="flex h-8 items-center gap-1.5 rounded-md bg-amber-400 px-3 text-xs font-semibold text-black transition hover:bg-amber-300"
      onclick={onAddWorkItem}
      type="button"
    >
      <Plus class="size-3.5" />
      Add work item
    </button>
  </div>
</div>
