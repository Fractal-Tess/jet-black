<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { useAuth, useMutation, useQuery } from "convex-svelte";
import Plus from "lucide-svelte/icons/plus";
import SettingsHeading from "$lib/components/settings/SettingsHeading.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";
import LabelRow from "./LabelRow.svelte";
import { STATE_COLOR_PALETTE } from "./state-colors";

let {
  project,
  role,
}: {
  project: { _id: string };
  role: WorkspaceRole;
} = $props();

const canManage = $derived(role !== "guest");
const auth = useAuth();
const projectId = $derived(project._id as Id<"projects">);

const labelsQuery = useQuery(api.queries.labels.listForProject, () =>
  auth.isAuthenticated ? { projectId } : "skip"
);

const createLabel = useMutation(api.mutations.labels.create);
const updateLabel = useMutation(api.mutations.labels.update);
const removeLabel = useMutation(api.mutations.labels.remove);

const labels = $derived(
  [...(labelsQuery.data ?? [])].sort((a, b) => a.name.localeCompare(b.name))
);

let creating = $state(false);
let newName = $state("");
let newColor = $state(STATE_COLOR_PALETTE[0]);
let error = $state("");

function startCreate() {
  creating = true;
  newName = "";
  newColor = STATE_COLOR_PALETTE[0];
  error = "";
}

async function submitCreate() {
  const name = newName.trim();
  if (!name) {
    error = "Name is required";
    return;
  }

  try {
    await createLabel({ color: newColor, name, projectId });
    creating = false;
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not create label.";
  }
}
</script>

<SettingsHeading
  title="Labels"
  description="Create labels to organize and filter issues."
/>

{#if error}
  <p class="mt-4 text-sm text-destructive" role="alert">{error}</p>
{/if}

<div class="mt-6 rounded-lg border border-border bg-card">
  <div
    class="flex items-center justify-between border-b border-border px-4 py-2.5"
  >
    <h4 class="text-sm font-medium text-foreground">Labels</h4>
    {#if canManage}
      <button
        class="flex h-7 items-center gap-1 rounded-md border border-border px-2 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
        onclick={startCreate}
        type="button"
      >
        <Plus class="size-3.5" />
        Add
      </button>
    {/if}
  </div>

  <div class="divide-y divide-border">
    {#each labels as label (label._id)}
      <LabelRow
        {canManage}
        {label}
        onDelete={async () => {
          await removeLabel({ labelId: label._id });
        }}
        onSave={async (input) => {
          await updateLabel({ ...input, labelId: label._id });
        }}
      />
    {/each}

    {#if creating}
      <div class="flex flex-col gap-2 px-4 py-3">
        <div class="flex items-center gap-2">
          <input
            bind:value={newName}
            class="h-8 flex-1 rounded-md border border-border bg-background px-2 text-sm text-foreground focus:border-ring focus:outline-none"
            placeholder="Label name"
            type="text"
          />
          <button
            class="h-8 rounded-md bg-primary px-2.5 text-xs font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
            disabled={!newName.trim()}
            onclick={submitCreate}
            type="button"
          >
            Create
          </button>
          <button
            class="h-8 rounded-md border border-border px-2.5 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
            onclick={() => (creating = false)}
            type="button"
          >
            Cancel
          </button>
        </div>
        <div class="flex flex-wrap items-center gap-1.5">
          {#each STATE_COLOR_PALETTE as swatch (swatch)}
            <button
              aria-label="Use color {swatch}"
              class="size-5 rounded-full border-2 {newColor === swatch
                ? 'border-foreground'
                : 'border-transparent'}"
              onclick={() => (newColor = swatch)}
              style="background-color:{swatch}"
              type="button"
            ></button>
          {/each}
        </div>
      </div>
    {/if}

    {#if labels.length === 0 && !creating}
      <p class="px-4 py-3 text-xs text-muted-foreground">
        No labels yet.
      </p>
    {/if}
  </div>
</div>
