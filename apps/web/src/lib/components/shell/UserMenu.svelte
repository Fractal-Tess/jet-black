<script lang="ts">
import LogOut from "lucide-svelte/icons/log-out";
import Settings from "lucide-svelte/icons/settings";
import Settings2 from "lucide-svelte/icons/settings-2";
import { goto } from "$app/navigation";
import { authClient } from "$lib/auth-client";
import type { WorkspaceUser } from "$lib/components/issues/types";
import { profileSettingsHref } from "$lib/routes";

let {
  user,
  onBeforeAuthExit,
}: {
  user: WorkspaceUser;
  onBeforeAuthExit?: () => void;
} = $props();

let open = $state(false);
let signingOut = $state(false);

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

function handleNav(href: string) {
  open = false;
  goto(href);
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    open = false;
  }
}
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

<div class="relative">
  <button
    aria-expanded={open}
    aria-haspopup="true"
    aria-label="User menu"
    class="grid size-7 place-items-center overflow-hidden rounded-full bg-primary text-meta font-bold text-primary-foreground transition hover:opacity-90"
    onclick={() => (open = !open)}
    title={user.email}
    type="button"
  >
    {#if user.image}
      <img
        alt={user.name || "Avatar"}
        class="size-full object-cover"
        src={user.image}
      />
    {:else}
      {initials}
    {/if}
  </button>

  {#if open}
    <!-- Backdrop -->
    <button
      aria-label="Close menu"
      class="fixed inset-0 z-40"
      onclick={() => (open = false)}
      type="button"
    ></button>

    <!-- Menu panel -->
    <div
      class="absolute right-0 top-10 z-50 flex w-72 flex-col overflow-hidden rounded-lg border border-border bg-card shadow-2xl"
    >
      <!-- Profile card -->
      <div class="relative flex flex-col items-center bg-muted px-4 pb-4 pt-8">
        <div
          class="grid size-10 place-items-center overflow-hidden rounded-full bg-primary text-sm font-bold text-primary-foreground"
        >
          {#if user.image}
            <img
              alt={user.name || "Avatar"}
              class="size-full object-cover"
              src={user.image}
            />
          {:else}
            {initials}
          {/if}
        </div>
        <p class="mt-2 max-w-full truncate text-sm font-medium text-foreground">
          {user.name || "Jet Black user"}
        </p>
        <p class="max-w-full truncate text-xs text-muted-foreground">
          {user.email}
        </p>
      </div>

      <!-- Menu items -->
      <div class="flex flex-col p-1.5">
        <button
        class="flex items-center gap-2.5 rounded-md px-3 py-2 text-sm text-secondary-foreground transition hover:bg-accent"
          onclick={() => handleNav(profileSettingsHref("general"))}
          type="button"
        >
          <Settings class="size-4 text-muted-foreground" />
          Settings
        </button>

        <button
        class="flex items-center gap-2.5 rounded-md px-3 py-2 text-sm text-secondary-foreground transition hover:bg-accent"
          onclick={() => handleNav(profileSettingsHref("preferences"))}
          type="button"
        >
          <Settings2 class="size-4 text-muted-foreground" />
          Preferences
        </button>

        <div class="my-1 border-t border-border"></div>

        <button
        class="flex items-center gap-2.5 rounded-md px-3 py-2 text-sm text-destructive transition hover:bg-destructive/10"
          disabled={signingOut}
          onclick={signOut}
          type="button"
        >
          <LogOut class="size-4" />
          {signingOut ? "Signing out…" : "Sign out"}
        </button>
      </div>
    </div>
  {/if}
</div>
