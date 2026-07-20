<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { useMutation } from "convex-svelte";
import { untrack } from "svelte";
import { goto } from "$app/navigation";
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsDangerZone from "$lib/components/settings/SettingsDangerZone.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

let {
  workspace,
  role,
}: {
  workspace: { _id: string; name: string; slug: string; updatedAt: number };
  role: WorkspaceRole;
} = $props();

const isAdmin = $derived(role === "owner" || role === "admin");
const isOwner = $derived(role === "owner");

const renameWorkspace = useMutation(api.mutations.workspaces.rename);
const removeWorkspace = useMutation(api.mutations.workspaces.remove);

let name = $state(untrack(() => workspace.name));
let saving = $state(false);
let saved = $state(false);
let error = $state("");
let deleteOpen = $state(false);
let deleteConfirmName = $state("");
let deleting = $state(false);

$effect(() => {
  name = workspace.name;
});

async function save() {
  const trimmed = name.trim();

  if (!trimmed || trimmed === workspace.name) {
    return;
  }

  saving = true;
  saved = false;
  error = "";

  try {
    await renameWorkspace({
      name: trimmed,
      workspaceId: workspace._id as Id<"workspaces">,
    });
    saved = true;
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Could not update the workspace.";
  } finally {
    saving = false;
  }
}

async function destroy() {
  if (deleteConfirmName !== workspace.name) {
    return;
  }

  deleting = true;
  error = "";

  try {
    await removeWorkspace({ workspaceId: workspace._id as Id<"workspaces"> });
    await goto("/dashboard");
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Could not delete the workspace.";
    deleting = false;
  }
}
</script>

<SettingsHeading
  title="General"
  description="Manage your workspace details."
/>

<div
  class="mt-8 flex flex-col gap-6 {isAdmin ? '' : 'opacity-60'}"
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
      disabled={!isAdmin}
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

  {#if error}
    <p class="text-sm text-destructive" role="alert">{error}</p>
  {/if}

  {#if isAdmin}
    <div class="flex items-center gap-3">
      <button
        class="h-9 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
        disabled={saving || !name.trim() || name.trim() === workspace.name}
        onclick={save}
        type="button"
      >
        {saving ? "Saving…" : "Update workspace"}
      </button>
      {#if saved}
        <span class="text-xs text-muted-foreground">Saved.</span>
      {/if}
    </div>
  {/if}
</div>

{#if isOwner}
  <SettingsDangerZone>
    <SettingsBoxedControl
      danger
      title="Delete workspace"
      description="This will permanently delete the workspace and all its data."
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
                disabled={deleting || deleteConfirmName !== workspace.name}
                onclick={destroy}
                type="button"
              >
                {deleting ? "Deleting…" : "Delete workspace"}
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
