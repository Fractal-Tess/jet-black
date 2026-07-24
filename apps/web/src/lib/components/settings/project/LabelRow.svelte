<script lang="ts">
import Trash2 from "lucide-svelte/icons/trash-2";
import type { IssueLabel } from "$lib/components/issues/types";
import { STATE_COLOR_PALETTE } from "./state-colors";

let {
  label,
  canManage,
  onSave,
  onDelete,
}: {
  label: IssueLabel;
  canManage: boolean;
  onSave: (input: { name: string; color: string }) => Promise<void>;
  onDelete: () => Promise<void>;
} = $props();

let editing = $state(false);
let name = $state("");
let color = $state("");
let busy = $state(false);
let error = $state("");

$effect(() => {
  if (editing) {
    return;
  }

  name = label.name;
  color = label.color;
});

function startEdit() {
  name = label.name;
  color = label.color;
  error = "";
  editing = true;
}

async function save() {
  const trimmed = name.trim();
  if (!trimmed) {
    error = "Name is required";
    return;
  }

  busy = true;
  error = "";
  try {
    await onSave({ color, name: trimmed });
    editing = false;
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not save.";
  } finally {
    busy = false;
  }
}

async function remove() {
  busy = true;
  error = "";
  try {
    await onDelete();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Could not delete.";
  } finally {
    busy = false;
  }
}
</script>

<div class="flex flex-col gap-2 px-4 py-2.5">
  <div class="flex items-center gap-3">
    <span
      aria-hidden="true"
      class="size-3 rounded-full"
      style="background-color:{label.color}"
    ></span>

    {#if editing}
      <input
        bind:value={name}
        class="h-8 flex-1 rounded-md border border-border bg-card px-2 text-sm text-foreground focus:border-ring focus:outline-none"
        type="text"
      />
    {:else}
      <span class="flex-1 text-sm text-foreground">{label.name}</span>
    {/if}

    {#if canManage}
      <div class="flex items-center gap-0.5">
        {#if editing}
          <button
            class="h-8 rounded-md bg-primary px-2.5 text-xs font-medium text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
            disabled={busy}
            onclick={save}
            type="button"
          >
            Save
          </button>
          <button
            class="h-8 rounded-md border border-border px-2.5 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
            onclick={() => (editing = false)}
            type="button"
          >
            Cancel
          </button>
        {:else}
          <button
            class="h-7 rounded-md px-2 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
            onclick={startEdit}
            type="button"
          >
            Edit
          </button>
          <button
            aria-label="Delete label"
            class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-destructive/10 hover:text-destructive"
            disabled={busy}
            onclick={remove}
            type="button"
          >
            <Trash2 class="size-3.5" />
          </button>
        {/if}
      </div>
    {/if}
  </div>

  {#if editing}
    <div class="flex flex-wrap items-center gap-1.5 pl-6">
      {#each STATE_COLOR_PALETTE as swatch (swatch)}
        <button
          aria-label="Use color {swatch}"
          class="size-5 rounded-full border-2 {color === swatch
            ? 'border-foreground'
            : 'border-transparent'}"
          onclick={() => (color = swatch)}
          style="background-color:{swatch}"
          type="button"
        ></button>
      {/each}
    </div>
  {/if}

  {#if error}
    <p class="pl-6 text-xs text-destructive" role="alert">{error}</p>
  {/if}
</div>
