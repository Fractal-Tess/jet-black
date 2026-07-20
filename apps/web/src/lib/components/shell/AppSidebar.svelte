<script lang="ts">
import type { Project } from "$lib/components/issues/types";
import CreateProjectModal from "$lib/components/projects/CreateProjectModal.svelte";
import type { ProjectModule, WorkspacePage } from "$lib/routes";
import SidebarNavigation from "./SidebarNavigation.svelte";

let {
  activeModule = "tickets",
  activeWorkspacePage,
  activeWorkspaceSlug,
  creatingProject = false,
  onCreateProject,
  onCloseSidebar,
  onSelectProject,
  projects,
  selectedProjectId,
  sidebarOpen = false,
}: {
  activeModule?: ProjectModule;
  activeWorkspacePage?: WorkspacePage | null;
  activeWorkspaceSlug?: string;
  creatingProject?: boolean;
  onCreateProject: (input: {
    description?: string;
    key: string;
    name: string;
  }) => Promise<boolean>;
  onCloseSidebar?: () => void;
  onSelectProject: (projectId: Project["_id"]) => void;
  projects: Project[];
  selectedProjectId?: string;
  sidebarOpen?: boolean;
} = $props();

let createProjectOpen = $state(false);
</script>

{#if sidebarOpen}
  <button
    aria-label="Close navigation"
    class="fixed inset-0 z-30 bg-background/60 lg:hidden"
    onclick={onCloseSidebar}
    type="button"
  ></button>
{/if}

<aside
  class="fixed bottom-0 left-0 top-12 z-40 flex w-[248px] flex-col border-r border-border bg-sidebar transition-transform lg:translate-x-0 {sidebarOpen
    ? 'translate-x-0'
    : '-translate-x-full'}"
>
  <div class="flex items-center justify-between px-4 py-3.5">
        <span class="text-base font-semibold text-foreground">Projects</span>
    <button
      aria-label="Close navigation"
      class="text-muted-foreground lg:hidden"
      onclick={onCloseSidebar}
      type="button"
    >
      ✕
    </button>
  </div>

  <SidebarNavigation
    {activeModule}
    {activeWorkspacePage}
    {activeWorkspaceSlug}
    onOpenCreateProject={() => (createProjectOpen = true)}
    {onSelectProject}
    {projects}
    {selectedProjectId}
  />
</aside>

{#if createProjectOpen && activeWorkspaceSlug}
  <CreateProjectModal
    {creatingProject}
    {onCreateProject}
    onClose={() => (createProjectOpen = false)}
    workspaceSlug={activeWorkspaceSlug}
  />
{/if}
