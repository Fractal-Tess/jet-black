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
  project,
  role,
  workspaceSlug,
}: {
  project: {
    _id: string;
    archivedAt?: number;
    color: string;
    coverImageUrl?: string;
    description?: string;
    key: string;
    logoUrl?: string;
    name: string;
    slug: string;
  };
  role: WorkspaceRole;
  workspaceSlug: string;
} = $props();

const isAdmin = $derived(role === "owner" || role === "admin");
const projectId = $derived(project._id as Id<"projects">);

const updateProject = useMutation(api.mutations.projects.update);
const archiveProject = useMutation(api.mutations.projects.archive);
const restoreProject = useMutation(api.mutations.projects.restore);
const removeProject = useMutation(api.mutations.projects.remove);
const generateUploadUrl = useMutation(api.mutations.projects.generateUploadUrl);
const storeProjectImage = useMutation(api.mutations.projects.storeProjectImage);

let name = $state(untrack(() => project.name));
let description = $state(untrack(() => project.description ?? ""));
let identifier = $state(untrack(() => project.key));
let saving = $state(false);
let saved = $state(false);
let error = $state("");
let deleteOpen = $state(false);
let deleteConfirmName = $state("");
let busy = $state(false);

$effect(() => {
  name = project.name;
  description = project.description ?? "";
  identifier = project.key;
});

async function save() {
  saving = true;
  saved = false;
  error = "";

  try {
    await updateProject({
      description: description.trim() || null,
      key: identifier,
      name,
      projectId,
    });
    saved = true;
  } catch (cause) {
    error =
      cause instanceof Error ? cause.message : "Could not update the project.";
  } finally {
    saving = false;
  }
}

async function uploadImage(file: File, type: "cover" | "logo") {
  error = "";
  try {
    const uploadUrl = await generateUploadUrl({});
    const response = await fetch(uploadUrl, {
      body: file,
      headers: { "Content-Type": file.type },
      method: "POST",
    });

    if (!response.ok) {
      throw new Error("Upload failed");
    }

    const { storageId } = await response.json();
    await storeProjectImage({ projectId, storageId, type });
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not upload image.";
  }
}

function onFileChange(event: Event, type: "cover" | "logo") {
  const input = event.currentTarget as HTMLInputElement;
  const file = input.files?.[0];
  if (file) {
    uploadImage(file, type);
  }
  input.value = "";
}

async function toggleArchive() {
  busy = true;
  error = "";
  try {
    if (project.archivedAt) {
      await restoreProject({ projectId });
    } else {
      await archiveProject({ projectId });
    }
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not archive.";
  } finally {
    busy = false;
  }
}

async function destroy() {
  if (deleteConfirmName !== project.name) {
    return;
  }

  busy = true;
  error = "";
  try {
    await removeProject({ projectId });
    await goto(`/workspace/${workspaceSlug}/settings/projects`);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not delete.";
    busy = false;
  }
}
</script>

<SettingsHeading
  title="General"
  description="Manage project details and configuration."
/>

<!-- Cover + logo area -->
<div
  class="relative mt-6 h-36 w-full overflow-hidden rounded-lg bg-muted"
  style={project.coverImageUrl
    ? `background-image:url(${project.coverImageUrl});background-size:cover;background-position:center`
    : ""}
>
  <div
    class="absolute inset-0 bg-gradient-to-t from-background/50 to-transparent"
  ></div>
  <div class="absolute bottom-4 left-4 flex items-center gap-3">
    {#if project.logoUrl}
      <img
        alt="{project.name} logo"
        class="size-10 rounded-lg border border-border object-cover"
        src={project.logoUrl}
      />
    {:else}
      <div
        class="grid size-10 place-items-center rounded-lg border border-primary/30 bg-primary text-sm font-bold text-primary-foreground"
      >
        {project.key.slice(0, 2)}
      </div>
    {/if}
    <div>
      <p class="text-sm font-medium text-foreground">{project.name}</p>
      <p class="text-xs text-muted-foreground">{project.key}</p>
    </div>
  </div>

  {#if isAdmin}
    <div class="absolute right-4 top-4 flex gap-2">
      <label
        class="cursor-pointer rounded-md border border-border bg-card/80 px-2.5 py-1 text-xs text-foreground transition hover:bg-card"
      >
        Cover
        <input
          accept="image/*"
          class="sr-only"
          onchange={(event) => onFileChange(event, "cover")}
          type="file"
        />
      </label>
      <label
        class="cursor-pointer rounded-md border border-border bg-card/80 px-2.5 py-1 text-xs text-foreground transition hover:bg-card"
      >
        Logo
        <input
          accept="image/*"
          class="sr-only"
          onchange={(event) => onFileChange(event, "logo")}
          type="file"
        />
      </label>
    </div>
  {/if}
</div>

<form
  class="mt-8 flex flex-col gap-6 {isAdmin ? '' : 'opacity-60'}"
  onsubmit={(e) => {
    e.preventDefault();
    save();
  }}
>
  <div class="grid grid-cols-1 gap-6 md:grid-cols-2">
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

  {#if error}
    <p class="text-sm text-destructive" role="alert">{error}</p>
  {/if}

  {#if isAdmin}
    <div class="flex items-center gap-3">
      <button
        class="h-9 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
        disabled={saving || !name.trim() || !identifier.trim()}
        type="submit"
      >
        {saving ? "Saving…" : "Update project"}
      </button>
      {#if saved}
        <span class="text-xs text-muted-foreground">Saved.</span>
      {/if}
    </div>
  {/if}
</form>

{#if isAdmin}
  <SettingsDangerZone>
    <SettingsBoxedControl
      danger
      title={project.archivedAt ? "Restore project" : "Archive project"}
      description={project.archivedAt
        ? "This project is archived. Restore it to make it active again."
        : "Archive this project. It can be restored later."}
    >
      {#snippet control()}
        <button
          class="h-8 rounded-md border border-border px-3 text-xs font-medium text-secondary-foreground transition hover:bg-accent disabled:opacity-50"
          disabled={busy}
          onclick={toggleArchive}
          type="button"
        >
          {project.archivedAt ? "Restore project" : "Archive project"}
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
                disabled={busy || deleteConfirmName !== project.name}
                onclick={destroy}
                type="button"
              >
                {busy ? "Deleting…" : "Delete project"}
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
