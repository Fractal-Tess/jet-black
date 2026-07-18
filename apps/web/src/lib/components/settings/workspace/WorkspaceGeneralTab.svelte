<script lang="ts">
import { untrack } from "svelte";
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsDangerZone from "$lib/components/settings/SettingsDangerZone.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";

let {
  workspace,
  role,
}: {
  workspace: { _id: string; name: string; slug: string; updatedAt: number };
  role: "owner" | "member";
} = $props();

const isOwner = $derived(role === "owner");

let name = $state(untrack(() => workspace.name));
let deleteOpen = $state(false);
let deleteConfirmName = $state("");

$effect(() => {
  name = workspace.name;
});
</script>

<SettingsHeading
  title="General"
  description="Manage your workspace details."
/>

<div
  class="mt-8 flex flex-col gap-6 {isOwner ? '' : 'opacity-60'}"
>
  <!-- Workspace logo -->
  <div class="flex items-center gap-4">
    <div
      class="grid size-14 place-items-center rounded-lg border border-border bg-muted text-lg font-bold text-foreground"
    >
      {workspace.name
        .trim()
        .split(/\s+/)
        .slice(0, 2)
        .map((w) => w[0])
        .join("")
        .toUpperCase()}
    </div>
    <div>
      <p class="text-sm font-medium text-foreground">{workspace.name}</p>
      <p class="text-xs text-muted-foreground">{workspace.slug}</p>
    </div>
  </div>

  <!-- Name -->
  <div class="flex flex-col gap-1.5">
    <label class="text-sm font-medium text-foreground" for="ws-name">
      Workspace name
    </label>
    <input
      bind:value={name}
      class="h-9 w-full max-w-sm rounded-md border border-border bg-card px-3 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
      disabled={!isOwner}
      id="ws-name"
      placeholder="Workspace name"
      type="text"
    />
  </div>

  <!-- URL (read-only) -->
  <div class="flex flex-col gap-1.5">
    <label class="text-sm font-medium text-foreground" for="ws-url">
      Workspace URL
    </label>
    <input
      class="h-9 w-full max-w-sm cursor-not-allowed rounded-md border border-border bg-muted px-3 text-sm text-muted-foreground"
      disabled
      id="ws-url"
      type="text"
      value="{typeof window !== 'undefined'
        ? window.location.origin
        : ''}/workspace/{workspace.slug}"
    />
  </div>

  <!-- Save (disabled until workspace update mutation is implemented) -->
  {#if isOwner}
    <div class="flex items-center gap-3">
      <button
        class="h-9 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
        disabled={true}
        title="Workspace updates coming soon"
        type="button"
      >
        Update workspace
      </button>
      <span class="text-xs text-muted-foreground">Coming soon</span>
    </div>
  {/if}
</div>

{#if isOwner}
  <SettingsDangerZone>
    <SettingsBoxedControl
      danger
      title="Delete workspace"
      description="This will permanently delete the workspace and all its data. Deletion is not yet available."
    >
      {#snippet control()}
        {#if deleteOpen}
          <div class="flex flex-col gap-2">
            <p class="text-xs text-muted-foreground">
              Type <strong class="text-foreground">{workspace.name}</strong> to
              confirm.
            </p>
            <input
              bind:value={deleteConfirmName}
              class="h-8 w-48 rounded-md border border-destructive/50 bg-card px-3 text-sm text-foreground focus:outline-none"
              placeholder={workspace.name}
              type="text"
            />
            <div class="flex gap-2">
              <button
                class="h-8 rounded-md bg-destructive px-3 text-xs font-semibold text-destructive-foreground transition hover:opacity-90 disabled:opacity-50"
                disabled={true}
                title="Workspace deletion coming soon"
                type="button"
              >
                Delete workspace
              </button>
              <button
                class="h-8 rounded-md border border-border px-3 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
                onclick={() => {
                  deleteOpen = false;
                  deleteConfirmName = "";
                }}
                type="button"
              >
                Cancel
              </button>
            </div>
          </div>
        {:else}
          <button
            class="h-8 rounded-md border border-destructive/50 px-3 text-xs font-medium text-destructive transition hover:bg-destructive/10"
            onclick={() => (deleteOpen = true)}
            type="button"
          >
            Delete workspace
          </button>
        {/if}
      {/snippet}
    </SettingsBoxedControl>
  </SettingsDangerZone>
{/if}
