<script lang="ts">
import Check from "lucide-svelte/icons/check";
import ChevronDown from "lucide-svelte/icons/chevron-down";
import LogOut from "lucide-svelte/icons/log-out";
import Plus from "lucide-svelte/icons/plus";
import Settings from "lucide-svelte/icons/settings";
import Users from "lucide-svelte/icons/users";
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";
import { workspaceHref, workspaceSettingsHref } from "$lib/routes";

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
  activeWorkspace,
  workspaces,
  memberships,
  userEmail = "",
  onBeforeAuthExit,
}: {
  activeWorkspace?: Workspace | null;
  workspaces: Workspace[];
  memberships: Membership[];
  userEmail?: string;
  onBeforeAuthExit?: () => void;
} = $props();

let open = $state(false);
let signingOut = $state(false);
let triggerEl: HTMLButtonElement | undefined = $state();
let menuTop = $state(0);
let menuLeft = $state(0);

const otherWorkspaces = $derived(
  workspaces.filter((w) => w._id !== activeWorkspace?._id)
);

function getRole(workspaceId: string) {
  return memberships.find((m) => m.workspaceId === workspaceId)?.role ?? "";
}

function getInitial(name: string) {
  return name.trim().charAt(0).toUpperCase();
}

function toggle() {
  if (!open && triggerEl) {
    const rect = triggerEl.getBoundingClientRect();
    menuTop = rect.bottom + 4;
    menuLeft = rect.left;
  }
  open = !open;
}

function close() {
  open = false;
}

function handleWorkspaceClick(workspace: Workspace) {
  close();
  goto(workspaceHref(workspace.slug));
}

async function handleSignOut() {
  signingOut = true;
  try {
    onBeforeAuthExit?.();
    await authClient.signOut();
    close();
    await goto("/login");
  } finally {
    signingOut = false;
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    close();
  }
}

function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
}
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

<button
  aria-expanded={open}
  aria-haspopup="true"
  bind:this={triggerEl}
  class="flex min-w-0 items-center gap-2 rounded-md px-1 py-1 transition hover:bg-accent"
  onclick={toggle}
  type="button"
>
  <span
    class="grid size-7 shrink-0 place-items-center rounded-md border border-primary/30 bg-primary/5"
  >
    <img
      alt=""
      class="size-5 object-contain"
      height="20"
      src="/brand/logo-mark-light.png"
      width="20"
    />
  </span>
  <span class="max-w-[120px] truncate text-sm font-medium text-foreground">
    {activeWorkspace?.name ?? "Jet Black"}
  </span>
  <ChevronDown
    class="size-3.5 shrink-0 text-muted-foreground transition-transform {open
      ? 'rotate-180'
      : ''}"
  />
</button>

{#if open}
  <div use:portal>
    <button
      aria-label="Close menu"
      class="fixed inset-0"
      onclick={close}
      style="z-index: 9998;"
      type="button"
    ></button>

    <div
      class="fixed flex w-[17rem] flex-col overflow-hidden rounded-lg border border-border bg-card shadow-2xl"
      style="z-index: 9999; top: {menuTop}px; left: {menuLeft}px;"
    >
      <!-- Current workspace section -->
      {#if activeWorkspace}
        <div class="p-2">
          <div class="rounded-md bg-accent px-3 py-2.5">
            <div class="flex items-center gap-2.5">
              <span
                class="grid size-8 shrink-0 place-items-center rounded-sm bg-primary text-sm font-bold text-primary-foreground"
              >
                {getInitial(activeWorkspace.name)}
              </span>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium text-foreground">
                  {activeWorkspace.name}
                </p>
                <p class="text-xs capitalize text-muted-foreground">
                  {getRole(activeWorkspace._id)}
                </p>
              </div>
              <Check class="size-4 shrink-0 text-primary" />
            </div>
            <!-- Inline quick actions -->
            <div class="mt-2 flex gap-1.5">
              <a
                class="flex h-6 items-center gap-1 rounded border border-border bg-card/80 px-1.5 text-[10px] text-muted-foreground transition hover:text-foreground"
                href={workspaceSettingsHref(activeWorkspace.slug)}
                onclick={(e) => {
                  e.stopPropagation();
                  close();
                }}
              >
                <Settings class="size-2.5" />
                Settings
              </a>
              <a
                class="flex h-6 items-center gap-1 rounded border border-border bg-card/80 px-1.5 text-[10px] text-muted-foreground transition hover:text-foreground"
                href={workspaceSettingsHref(activeWorkspace.slug, "members")}
                onclick={(e) => {
                  e.stopPropagation();
                  close();
                }}
              >
                <Users class="size-2.5" />
                Members
              </a>
            </div>
          </div>
        </div>
      {/if}

      <!-- Other workspaces -->
      {#if otherWorkspaces.length > 0}
        <div class="border-t border-border px-2 py-1.5">
          <p class="px-2 py-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
            Switch to
          </p>
          {#each otherWorkspaces as workspace (workspace._id)}
            <button
              class="flex w-full items-center gap-2.5 rounded-md px-2 py-2 text-left transition hover:bg-accent"
              onclick={() => handleWorkspaceClick(workspace)}
              type="button"
            >
              <span
                class="grid size-7 shrink-0 place-items-center rounded-sm bg-muted text-xs font-bold text-foreground"
              >
                {getInitial(workspace.name)}
              </span>
              <div class="min-w-0">
                <p class="truncate text-sm text-secondary-foreground">
                  {workspace.name}
                </p>
                <p class="text-[11px] capitalize text-muted-foreground">
                  {getRole(workspace._id)}
                </p>
              </div>
            </button>
          {/each}
        </div>
      {/if}

      <!-- Actions -->
      <div class="border-t border-border p-1.5">
        <a
          class="flex items-center gap-2 rounded-md px-2.5 py-1.5 text-[13px] text-secondary-foreground transition hover:bg-accent"
          href="/onboarding?step=workspace"
          onclick={close}
        >
          <Plus class="size-3.5 text-muted-foreground" />
          Create workspace
        </a>
        <button
          class="flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-[13px] text-muted-foreground transition hover:bg-accent hover:text-foreground"
          disabled={signingOut}
          onclick={handleSignOut}
          type="button"
        >
          <LogOut class="size-3.5" />
          {signingOut ? "Signing out…" : "Sign out"}
        </button>
      </div>

      <!-- Email footer -->
      <div class="border-t border-border px-3 py-2">
        <p class="truncate text-[11px] text-muted-foreground">{userEmail}</p>
      </div>
    </div>
  </div>
{/if}
