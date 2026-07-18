<script lang="ts">
import Search from "lucide-svelte/icons/search";
import type { Snippet } from "svelte";
import type { Project, WorkspaceUser } from "$lib/components/issues/types";
import type { ProjectModule } from "$lib/routes";
import AppSidebar from "./AppSidebar.svelte";
import UserMenu from "./UserMenu.svelte";
import WorkspaceSwitcher from "./WorkspaceSwitcher.svelte";

type Workspace = {
  _id: string;
  name: string;
  slug: string;
};

type Membership = {
  role: string;
  workspaceId: string;
};

let {
  activeModule = "tickets",
  activeWorkspace,
  activeWorkspaceSlug,
  children,
  connected,
  creatingProject = false,
  memberships = [],
  onBeforeAuthExit,
  onCreateProject,
  onSelectProject,
  projects,
  selectedProjectId,
  user,
  workspaces = [],
}: {
  activeModule?: ProjectModule;
  activeWorkspace?: Workspace | null;
  activeWorkspaceSlug?: string;
  children: Snippet;
  connected: boolean;
  creatingProject?: boolean;
  memberships?: Membership[];
  onBeforeAuthExit?: () => void;
  onCreateProject: (input: {
    description?: string;
    key: string;
    name: string;
  }) => Promise<boolean>;
  onSelectProject: (projectId: Project["_id"]) => void;
  projects: Project[];
  selectedProjectId?: string;
  user: WorkspaceUser;
  workspaces?: Workspace[];
} = $props();

let sidebarOpen = $state(false);
</script>

<div class="min-h-screen bg-background text-foreground">
  <header
    class="fixed inset-x-0 top-0 z-30 flex h-12 items-center border-b border-border bg-background"
  >
    <div
      class="flex h-full w-[248px] shrink-0 items-center border-r border-border px-3"
    >
      <button
        aria-label="Open navigation"
        class="mr-2 grid size-8 place-items-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground lg:hidden"
        onclick={() => (sidebarOpen = true)}
        type="button"
      >
        ☰
      </button>
      <WorkspaceSwitcher
        {activeWorkspace}
        {workspaces}
        {memberships}
        userEmail={user.email}
        {onBeforeAuthExit}
      />
    </div>

    <div class="flex min-w-0 flex-1 items-center justify-center px-3">
      <button
        class="flex h-8 w-full max-w-sm items-center gap-2 rounded-md border border-border bg-card px-3 text-left text-xs text-muted-foreground transition hover:border-input hover:text-secondary-foreground"
        type="button"
      >
        <span aria-hidden="true"><Search class="size-3.5" /></span>
        <span class="truncate">Search commands…</span>
        <kbd
          class="ml-auto hidden font-mono text-[10px] text-sidebar-muted-foreground sm:block"
        >
          ⌘ K
        </kbd>
      </button>
    </div>

    <div class="flex items-center gap-2 px-3">
      <span
        class="hidden items-center gap-1.5 rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground sm:flex"
      >
        <span
          class="size-1.5 rounded-full {connected
            ? 'bg-primary'
            : 'bg-muted-foreground'}"
        ></span>
        {connected ? "Live" : "Connecting"}
      </span>
      <UserMenu {user} {onBeforeAuthExit} />
    </div>
  </header>

  <AppSidebar
    {activeModule}
    {activeWorkspaceSlug}
    {creatingProject}
    {onCreateProject}
    {onSelectProject}
    {projects}
    {selectedProjectId}
    {sidebarOpen}
    onCloseSidebar={() => (sidebarOpen = false)}
  />

  <main class="min-h-screen pt-12 lg:pl-[248px]">
    {@render children()}
  </main>
</div>
