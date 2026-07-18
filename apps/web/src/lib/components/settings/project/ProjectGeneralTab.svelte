<script lang="ts">
import { untrack } from "svelte";
import SettingsBoxedControl from "$lib/components/settings/SettingsBoxedControl.svelte";
import SettingsDangerZone from "$lib/components/settings/SettingsDangerZone.svelte";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";

let {
  project,
  role,
}: {
  project: {
    _id: string;
    color: string;
    description?: string;
    key: string;
    name: string;
    slug: string;
  };
  role: "owner" | "member";
} = $props();

const isAdmin = $derived(role === "owner");

let name = $state(untrack(() => project.name));
let description = $state(untrack(() => project.description ?? ""));
let identifier = $state(untrack(() => project.key));

let deleteOpen = $state(false);
let deleteConfirmName = $state("");

$effect(() => {
  name = project.name;
  description = project.description ?? "";
  identifier = project.key;
});
</script>

<SettingsHeading
  title="General"
  description="Manage project details and configuration."
/>

<!-- Cover + logo area -->
<div class="relative mt-6 h-36 w-full overflow-hidden rounded-lg bg-muted">
  <div class="absolute inset-0 bg-gradient-to-t from-background/50 to-transparent"></div>
  <div class="absolute bottom-4 left-4 flex items-center gap-3">
    <div
      class="grid size-10 place-items-center rounded-lg border border-border text-sm font-bold"
      style:background-color={project.color}
    >
      {project.key.slice(0, 2)}
    </div>
    <div>
      <p class="text-sm font-medium text-foreground">{project.name}</p>
      <p class="text-xs text-muted-foreground">{project.key}</p>
    </div>
  </div>
</div>

<form
  class="mt-8 flex flex-col gap-6 {isAdmin ? '' : 'opacity-60'}"
  onsubmit={(e) => {
    e.preventDefault();
  }}
>
  <div class="grid grid-cols-1 gap-6 md:grid-cols-2">
    <!-- Name -->
    <div class="flex flex-col gap-1.5">
      <label class="text-sm font-medium text-foreground" for="proj-name">
        Project name
      </label>
      <input
        bind:value={name}
        class="h-9 w-full rounded-md border border-border bg-card px-3 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
        disabled={!isAdmin}
        id="proj-name"
        type="text"
      />
    </div>

    <!-- Identifier -->
    <div class="flex flex-col gap-1.5">
      <label class="text-sm font-medium text-foreground" for="proj-identifier">
        Identifier
      </label>
      <input
        bind:value={identifier}
        class="h-9 w-full rounded-md border border-border bg-card px-3 text-sm uppercase text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
        disabled={!isAdmin}
        id="proj-identifier"
        maxlength={10}
        type="text"
      />
      <p class="text-xs text-muted-foreground">
        Used as prefix for work item identifiers (e.g. {identifier}-1).
      </p>
    </div>
  </div>

  <!-- Description -->
  <div class="flex flex-col gap-1.5">
    <label class="text-sm font-medium text-foreground" for="proj-desc">
      Description
    </label>
    <textarea
      bind:value={description}
      class="min-h-[6rem] w-full rounded-md border border-border bg-card px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
      disabled={!isAdmin}
      id="proj-desc"
      placeholder="Describe this project…"
    ></textarea>
  </div>

  <!-- Save (disabled until project update mutation is implemented) -->
  {#if isAdmin}
    <div class="flex items-center gap-3">
      <button
        class="h-9 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
        disabled={true}
        title="Project updates coming soon"
        type="button"
      >
        Update project
      </button>
      <span class="text-xs text-muted-foreground">Coming soon</span>
    </div>
  {/if}
</form>

{#if isAdmin}
  <SettingsDangerZone>
    <SettingsBoxedControl
      danger
      title="Archive project"
      description="Archive this project. It can be restored later."
    >
      {#snippet control()}
        <button
          class="h-8 rounded-md border border-border px-3 text-xs font-medium text-secondary-foreground transition hover:bg-accent disabled:opacity-50"
          disabled={true}
          title="Archive coming soon"
          type="button"
        >
          Archive project
        </button>
      {/snippet}
    </SettingsBoxedControl>

    <SettingsBoxedControl
      danger
      title="Delete project"
      description="Permanently delete this project and all its data."
    >
      {#snippet control()}
        {#if deleteOpen}
          <div class="flex flex-col gap-2">
            <p class="text-xs text-muted-foreground">
              Type <strong class="text-foreground">{project.name}</strong> to confirm.
            </p>
            <input
              bind:value={deleteConfirmName}
              class="h-8 w-48 rounded-md border border-destructive/50 bg-card px-3 text-sm text-foreground focus:outline-none"
              placeholder={project.name}
              type="text"
            />
            <div class="flex gap-2">
              <button
                class="h-8 rounded-md bg-destructive px-3 text-xs font-semibold text-destructive-foreground transition hover:opacity-90 disabled:opacity-50"
                disabled={true}
                title="Project deletion coming soon"
                type="button"
              >
                Delete project
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
            Delete project
          </button>
        {/if}
      {/snippet}
    </SettingsBoxedControl>
  </SettingsDangerZone>
{/if}
