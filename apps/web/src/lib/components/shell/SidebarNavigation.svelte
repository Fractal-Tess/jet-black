<script lang="ts">
import ChevronRight from "lucide-svelte/icons/chevron-right";
import Copy from "lucide-svelte/icons/copy";
import FolderKanban from "lucide-svelte/icons/folder-kanban";
import Home from "lucide-svelte/icons/home";
import MoreHorizontal from "lucide-svelte/icons/more-horizontal";
import Plus from "lucide-svelte/icons/plus";
import Settings from "lucide-svelte/icons/settings";
import type { Project } from "$lib/components/issues/types";
import {
  type ProjectModule,
  projectModuleHref,
  workspaceHref,
  workspaceProjectsHref,
  workspaceSettingsHref,
} from "$lib/routes";
import { moduleIcons, moduleLinks } from "./sidebarConfig";

let {
  activeModule = "tickets",
  activeWorkspaceSlug,
  onOpenCreateProject,
  onSelectProject,
  projects,
  selectedProjectId,
}: {
  activeModule?: ProjectModule;
  activeWorkspaceSlug?: string;
  onOpenCreateProject: () => void;
  onSelectProject: (projectId: Project["_id"]) => void;
  projects: Project[];
  selectedProjectId?: string;
} = $props();

let workspaceMenuOpen = $state(true);
let workspaceOverflowOpen = $state(false);
let projectOverflowId = $state<string | null>(null);

const selectedProject = $derived(
  projects.find((project) => project._id === selectedProjectId) ?? null
);

function copyProjectLink(project: Project) {
  if (!activeWorkspaceSlug) {
    return;
  }
  const url = `${window.location.origin}${projectModuleHref({
    module: "tickets",
    projectId: project._id,
    workspaceSlug: activeWorkspaceSlug,
  })}`;
  navigator.clipboard.writeText(url);
  projectOverflowId = null;
}
</script>

{#if workspaceOverflowOpen || projectOverflowId}
  <button
    aria-label="Close menu"
    class="fixed inset-0 z-40"
    onclick={() => {
      workspaceOverflowOpen = false;
      projectOverflowId = null;
    }}
    type="button"
  ></button>
{/if}

<nav class="flex flex-1 flex-col gap-0.5 overflow-y-auto px-3 pb-2">
  <a
    class="flex h-8 items-center gap-2 rounded-md px-3 text-[13px] text-muted-foreground transition hover:bg-accent hover:text-foreground {activeModule ===
      'tickets' && !selectedProject
      ? 'bg-sidebar-accent text-sidebar-accent-foreground'
      : ''}"
    href={activeWorkspaceSlug
      ? workspaceHref(activeWorkspaceSlug)
      : "/dashboard"}
  >
    <Home class="size-4 text-sidebar-muted-foreground" />
    Home
  </a>

  <div class="group/workspace mt-3">
    <div class="flex items-center">
      <button
        aria-expanded={workspaceMenuOpen}
        class="flex flex-1 items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] font-semibold text-sidebar-muted-foreground transition hover:text-sidebar-foreground"
        onclick={() => (workspaceMenuOpen = !workspaceMenuOpen)}
        type="button"
      >
        <ChevronRight
          class="size-3.5 transition-transform {workspaceMenuOpen
            ? 'rotate-90'
            : ''}"
        />
        Workspace
      </button>

      {#if activeWorkspaceSlug}
        <div class="relative">
          <button
            aria-label="Workspace options"
            class="grid size-6 place-items-center rounded-md text-sidebar-muted-foreground opacity-0 transition hover:bg-sidebar-accent hover:text-sidebar-foreground group-hover/workspace:opacity-100"
            onclick={() =>
              (workspaceOverflowOpen = !workspaceOverflowOpen)}
            type="button"
          >
            <MoreHorizontal class="size-3.5" />
          </button>

          {#if workspaceOverflowOpen}
            <div
              class="absolute left-0 top-7 z-50 w-40 overflow-hidden rounded-md border border-border bg-card shadow-lg"
            >
              <a
                class="flex items-center gap-2 px-3 py-2 text-[13px] text-secondary-foreground transition hover:bg-accent"
                href={workspaceSettingsHref(activeWorkspaceSlug)}
                onclick={() => (workspaceOverflowOpen = false)}
              >
                <Settings class="size-3.5 text-muted-foreground" />
                Settings
              </a>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    {#if workspaceMenuOpen}
      <div class="mt-0.5 flex flex-col gap-0.5">
        <a
          class="flex h-8 items-center gap-2 rounded-md px-3 text-[13px] text-muted-foreground transition hover:bg-accent hover:text-foreground"
          href={activeWorkspaceSlug
            ? workspaceProjectsHref(activeWorkspaceSlug)
            : "/dashboard"}
        >
          <FolderKanban class="size-4 text-sidebar-muted-foreground" />
          Projects
        </a>
      </div>
    {/if}
  </div>

  <div class="mt-3">
    <div class="mb-1 flex items-center justify-between px-2">
      <p
        class="text-[11px] font-semibold uppercase tracking-wider text-sidebar-muted-foreground"
      >
        Projects
      </p>
      <button
        aria-label="Create project"
        class="grid size-5 place-items-center rounded text-sidebar-muted-foreground transition hover:bg-sidebar-accent hover:text-sidebar-foreground"
        onclick={onOpenCreateProject}
        type="button"
      >
        <Plus class="size-3" />
      </button>
    </div>
    <div class="flex flex-col gap-0.5">
      {#each projects as project (project._id)}
        <div class="group/project-item relative flex items-center">
          <a
            class="flex h-8 w-full items-center gap-2 rounded-md px-3 text-left text-[13px] text-muted-foreground transition hover:bg-accent hover:text-foreground {selectedProjectId ===
            project._id
              ? 'bg-sidebar-accent text-sidebar-accent-foreground'
              : ''}"
            href={activeWorkspaceSlug
              ? projectModuleHref({
                  module: "tickets",
                  projectId: project._id,
                  workspaceSlug: activeWorkspaceSlug,
                })
              : "/dashboard"}
            onclick={() => onSelectProject(project._id)}
          >
            <span
              class="size-2 shrink-0 rounded-sm bg-primary"
              style:background-color={project.color}
            ></span>
            <span class="truncate">{project.name}</span>
          </a>

          <div class="absolute right-1">
            <button
              aria-label="Project options"
              class="grid size-6 place-items-center rounded-md text-sidebar-muted-foreground opacity-0 transition hover:bg-sidebar-accent hover:text-sidebar-foreground group-hover/project-item:opacity-100"
              onclick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                projectOverflowId =
                  projectOverflowId === project._id
                    ? null
                    : project._id;
              }}
              type="button"
            >
              <MoreHorizontal class="size-3.5" />
            </button>

            {#if projectOverflowId === project._id}
              <div
                class="absolute right-0 top-7 z-50 w-40 overflow-hidden rounded-md border border-border bg-card shadow-lg"
              >
                <button
                  class="flex w-full items-center gap-2 px-3 py-2 text-left text-[13px] text-secondary-foreground transition hover:bg-accent"
                  onclick={() => copyProjectLink(project)}
                  type="button"
                >
                  <Copy class="size-3.5 text-muted-foreground" />
                  Copy link
                </button>
                {#if activeWorkspaceSlug}
                  <a
                    class="flex items-center gap-2 px-3 py-2 text-[13px] text-secondary-foreground transition hover:bg-accent"
                    href={`/workspace/${activeWorkspaceSlug}/settings/projects/${project._id}`}
                    onclick={() => (projectOverflowId = null)}
                  >
                    <Settings class="size-3.5 text-muted-foreground" />
                    Settings
                  </a>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div
          class="rounded-md border border-dashed border-border px-3 py-3 text-[12px] leading-5 text-muted-foreground"
        >
          Your projects will appear here.
        </div>
      {/each}
    </div>
  </div>

  {#if activeWorkspaceSlug && selectedProject}
    <div class="mt-3">
      <p
        class="mb-1 px-2 text-[11px] font-semibold uppercase tracking-wider text-sidebar-muted-foreground"
      >
        {selectedProject.name}
      </p>
      <div class="flex flex-col gap-0.5">
        {#each moduleLinks as item}
          {@const Icon = moduleIcons[item.module]}
          <a
            class="flex h-8 items-center gap-2 rounded-md px-3 text-[13px] text-muted-foreground transition hover:bg-accent hover:text-foreground {activeModule ===
              item.module ||
            (activeModule === 'issues' && item.module === 'tickets')
              ? 'bg-sidebar-accent text-sidebar-accent-foreground'
              : ''}"
            href={projectModuleHref({
              module: item.module,
              projectId: selectedProject._id,
              workspaceSlug: activeWorkspaceSlug,
            })}
          >
            <Icon
              class="size-4 flex-shrink-0 text-sidebar-muted-foreground"
            />
            {item.label}
          </a>
        {/each}
      </div>
    </div>
  {/if}
</nav>
