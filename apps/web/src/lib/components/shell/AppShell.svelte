<script lang="ts">
import type { Snippet } from "svelte";
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";
import type { Project, ViewerData } from "$lib/components/issues/types";

let {
  children,
  connected,
  creatingProject = false,
  onBeforeAuthExit,
  onCreateProject,
  onSelectProject,
  projects,
  selectedProjectId,
  user,
}: {
  children: Snippet;
  connected: boolean;
  creatingProject?: boolean;
  onBeforeAuthExit?: () => void;
  onCreateProject: (input: { key: string; name: string }) => Promise<boolean>;
  onSelectProject: (projectId: Project["_id"]) => void;
  projects: Project[];
  selectedProjectId?: string;
  user: ViewerData["user"];
} = $props();

let createProjectOpen = $state(false);
let projectName = $state("");
let projectKey = $state("");
let projectError = $state("");
let deleteAccountOpen = $state(false);
let deletingAccount = $state(false);
let deleteAccountError = $state("");
let signingOut = $state(false);
let sidebarOpen = $state(false);

const initials = $derived(
  user.name
    ?.trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toUpperCase() ?? user.email.slice(0, 2).toUpperCase()
);

async function signOut() {
  signingOut = true;

  try {
    onBeforeAuthExit?.();
    await authClient.signOut();
    await goto("/login");
  } finally {
    signingOut = false;
  }
}

async function deleteAccount() {
  deleteAccountError = "";
  deletingAccount = true;

  try {
    onBeforeAuthExit?.();
    const result = await authClient.deleteUser({
      callbackURL: "/login",
    });

    if (result.error) {
      deleteAccountError =
        result.error.message ?? "Could not delete your account.";
      return;
    }

    deleteAccountOpen = false;
    await goto("/login");
  } catch (error) {
    deleteAccountError =
      error instanceof Error ? error.message : "Could not delete your account.";
  } finally {
    deletingAccount = false;
  }
}

function deriveProjectKey(name: string) {
  return name
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "")
    .slice(0, 4);
}

async function createProject() {
  projectError = "";

  if (!projectName.trim()) {
    projectError = "Project name is required.";
    return;
  }

  const key = projectKey.trim() || deriveProjectKey(projectName);

  if (!key) {
    projectError = "Project key is required.";
    return;
  }

  try {
    const created = await onCreateProject({
      key,
      name: projectName,
    });

    if (!created) {
      projectError = "Could not create project yet. Try again in a moment.";
      return;
    }

    createProjectOpen = false;
    projectName = "";
    projectKey = "";
  } catch (error) {
    projectError =
      error instanceof Error ? error.message : "Could not create project.";
  }
}
</script>

<div class="min-h-screen bg-[#0d0e0e] text-zinc-200">
  <header
    class="fixed inset-x-0 top-0 z-30 flex h-12 items-center border-b border-white/10 bg-[#0d0e0e]"
  >
    <div
      class="flex h-full w-[248px] shrink-0 items-center border-r border-white/10 px-3"
    >
      <button
        aria-label="Open navigation"
        class="mr-2 grid size-8 place-items-center rounded-md text-zinc-400 hover:bg-white/5 hover:text-white lg:hidden"
        onclick={() => (sidebarOpen = true)}
        type="button"
      >
        ☰
      </button>
      <a class="flex min-w-0 items-center gap-2" href="/dashboard">
        <span
          class="grid size-7 shrink-0 place-items-center rounded-md border border-amber-400/30 bg-amber-400/5"
        >
          <span class="size-2.5 rotate-45 rounded-[2px] bg-amber-400"></span>
        </span>
        <span class="truncate text-sm font-medium">Jet Black</span>
        <span class="text-xs text-zinc-600">⌄</span>
      </a>
    </div>

    <div class="flex min-w-0 flex-1 items-center justify-center px-3">
      <button
        class="flex h-8 w-full max-w-sm items-center gap-2 rounded-md border border-white/10 bg-[#191a1a] px-3 text-left text-xs text-zinc-500 transition hover:border-white/20 hover:text-zinc-400"
        type="button"
      >
        <span aria-hidden="true">⌕</span>
        <span class="truncate">Search commands…</span>
        <kbd class="ml-auto hidden font-mono text-[10px] text-zinc-600 sm:block">
          ⌘ K
        </kbd>
      </button>
    </div>

    <div class="flex items-center gap-2 px-3">
      <span
        class="hidden items-center gap-1.5 rounded-md border border-white/10 px-2 py-1 text-[11px] text-zinc-500 sm:flex"
      >
        <span
          class="size-1.5 rounded-full {connected
            ? 'bg-emerald-400'
            : 'bg-zinc-600'}"
        ></span>
        {connected ? "Live" : "Connecting"}
      </span>
      <div
        class="grid size-7 place-items-center rounded-full bg-amber-400 text-[10px] font-bold text-black"
        title={user.email}
      >
        {initials}
      </div>
    </div>
  </header>

  {#if sidebarOpen}
    <button
      aria-label="Close navigation"
      class="fixed inset-0 z-30 bg-black/60 lg:hidden"
      onclick={() => (sidebarOpen = false)}
      type="button"
    ></button>
  {/if}

  <aside
    class="fixed bottom-0 left-0 top-12 z-40 flex w-[248px] flex-col border-r border-white/10 bg-[#111212] transition-transform lg:translate-x-0 {sidebarOpen
      ? 'translate-x-0'
      : '-translate-x-full'}"
  >
    <div class="flex items-center justify-between px-4 py-4">
      <h2 class="text-base font-medium">Projects</h2>
      <button
        aria-label="Close navigation"
        class="text-zinc-500 lg:hidden"
        onclick={() => (sidebarOpen = false)}
        type="button"
      >
        ✕
      </button>
    </div>

    <nav class="flex-1 space-y-5 overflow-y-auto px-3 pb-4 text-sm">
      <div class="space-y-1">
        <a
          class="mb-3 flex h-9 w-full items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] px-3 text-left text-zinc-300 transition hover:border-white/20 hover:bg-white/[0.06]"
          href="#new-issue"
        >
          <span aria-hidden="true">✧</span>
          New work item
        </a>
        <a
          class="flex h-8 items-center gap-2 rounded-md bg-white/10 px-3 text-zinc-100"
          href="/dashboard"
        >
          <span aria-hidden="true">⌂</span>
          Home
        </a>
      </div>

      <div>
        <p class="mb-2 px-2 text-xs font-medium text-zinc-600">Workspace</p>
        <a
          class="flex h-8 items-center gap-2 rounded-md px-3 text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200"
          href="/dashboard"
        >
          <span aria-hidden="true">▣</span>
          Projects
        </a>
        <a
          class="flex h-8 items-center gap-2 rounded-md px-3 text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200"
          href="/dashboard"
        >
          <span aria-hidden="true">•••</span>
          More
        </a>
      </div>

      <div>
        <div class="mb-2 flex items-center justify-between px-2">
          <p class="text-xs font-medium text-zinc-600">Projects</p>
          <button
            aria-label="Create project"
            class="grid size-6 place-items-center rounded-md text-zinc-500 transition hover:bg-white/5 hover:text-zinc-200"
            onclick={() => (createProjectOpen = !createProjectOpen)}
            type="button"
          >
            +
          </button>
        </div>
        {#if createProjectOpen}
          <form
            class="mb-3 space-y-2 rounded-md border border-white/10 bg-white/[0.03] p-2"
            onsubmit={(event) => event.preventDefault()}
          >
            <label class="block">
              <span class="sr-only">Project name</span>
              <input
                bind:value={projectName}
                class="h-8 w-full rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-amber-400/60"
                placeholder="Project name"
              />
            </label>
            <label class="block">
              <span class="sr-only">Project key</span>
              <input
                bind:value={projectKey}
                class="h-8 w-full rounded-md border border-white/10 bg-[#0f1010] px-2 font-mono text-xs uppercase text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-amber-400/60"
                placeholder={deriveProjectKey(projectName) || "KEY"}
              />
            </label>
            {#if projectError}
              <p class="text-[11px] text-red-300">{projectError}</p>
            {/if}
            <button
              class="h-8 w-full rounded-md bg-amber-400 px-3 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:cursor-not-allowed disabled:opacity-50"
              data-testid="create-project-submit"
              disabled={creatingProject || !projectName.trim()}
              onclick={createProject}
              type="button"
            >
              {creatingProject ? "Creating…" : "Create project"}
            </button>
          </form>
        {/if}
        <div class="space-y-1">
          {#each projects as project (project._id)}
            <button
              class="flex h-8 w-full items-center gap-2 rounded-md px-3 text-left text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200 {selectedProjectId ===
              project._id
                ? 'bg-white/10 text-zinc-100'
                : ''}"
              onclick={() => onSelectProject(project._id)}
              type="button"
            >
              <span
                class="size-2 rounded-sm"
                style:background-color={project.color}
              ></span>
              <span class="truncate">{project.name}</span>
            </button>
          {:else}
            <div
              class="rounded-md border border-dashed border-white/10 px-3 py-4 text-xs leading-5 text-zinc-600"
            >
              Your projects will appear here.
            </div>
          {/each}
        </div>
      </div>
    </nav>

    <div class="border-t border-white/10 p-3">
      <div class="mb-3 flex min-w-0 items-center gap-2 px-1">
        <div
          class="grid size-7 shrink-0 place-items-center rounded-full bg-amber-400 text-[10px] font-bold text-black"
        >
          {initials}
        </div>
        <div class="min-w-0">
          <p class="truncate text-xs font-medium text-zinc-300">
            {user.name || "Jet Black user"}
          </p>
          <p class="truncate text-[10px] text-zinc-600">{user.email}</p>
        </div>
      </div>
      <button
        class="flex h-8 w-full items-center rounded-md px-3 text-left text-xs text-zinc-500 transition hover:bg-white/5 hover:text-zinc-300 disabled:opacity-50"
        disabled={signingOut || deletingAccount}
        onclick={signOut}
        type="button"
      >
        {signingOut ? "Signing out…" : "Sign out"}
      </button>
      <button
        class="mt-1 flex h-8 w-full items-center rounded-md px-3 text-left text-xs text-red-400/80 transition hover:bg-red-500/10 hover:text-red-300 disabled:opacity-50"
        disabled={signingOut || deletingAccount}
        onclick={() => {
          deleteAccountError = "";
          deleteAccountOpen = true;
        }}
        type="button"
      >
        Delete account
      </button>
      {#if deleteAccountOpen}
        <div
          class="mt-2 rounded-md border border-red-400/20 bg-red-500/10 p-3"
        >
          <p class="text-xs leading-5 text-red-100">
            Delete your account and workspace data? This cannot be undone.
          </p>
          <div class="mt-3 flex gap-2">
            <button
              class="h-8 rounded-md bg-red-400 px-3 text-xs font-semibold text-black transition hover:bg-red-300 disabled:opacity-50"
              disabled={deletingAccount}
              onclick={deleteAccount}
              type="button"
            >
              {deletingAccount ? "Deleting…" : "Confirm delete account"}
            </button>
            <button
              class="h-8 rounded-md border border-white/10 px-3 text-xs text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200 disabled:opacity-50"
              disabled={deletingAccount}
              onclick={() => {
                deleteAccountError = "";
                deleteAccountOpen = false;
              }}
              type="button"
            >
              Cancel
            </button>
          </div>
        </div>
      {/if}
      {#if deleteAccountError}
        <p class="mt-2 px-3 text-[11px] text-red-300" role="alert">
          {deleteAccountError}
        </p>
      {/if}
    </div>
  </aside>

  <main class="min-h-screen pt-12 lg:pl-[248px]">
    {@render children()}
  </main>
</div>
