<script lang="ts">
import ChevronDown from "lucide-svelte/icons/chevron-down";
import ChevronUp from "lucide-svelte/icons/chevron-up";
import Star from "lucide-svelte/icons/star";
import Trash2 from "lucide-svelte/icons/trash-2";
import StateTypeIcon from "$lib/components/issues/StateTypeIcon.svelte";
import type { IssueState } from "$lib/components/issues/types";
import { STATE_COLOR_PALETTE } from "./state-colors";

let {
  issueState,
  canManage,
  isFirst,
  isLast,
  onSave,
  onSetDefault,
  onMoveUp,
  onMoveDown,
  onDelete,
}: {
  issueState: IssueState;
  canManage: boolean;
  isFirst: boolean;
  isLast: boolean;
  onSave: (input: { name: string; color: string }) => Promise<void>;
  onSetDefault: () => Promise<void>;
  onMoveUp: () => Promise<void>;
  onMoveDown: () => Promise<void>;
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

  name = issueState.name;
  color = issueState.color;
});

function startEdit() {
  name = issueState.name;
  color = issueState.color;
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

async function run(action: () => Promise<void>) {
  busy = true;
  error = "";
  try {
    await action();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "Something went wrong.";
  } finally {
    busy = false;
  }
}
</script>

<div class="flex flex-col gap-2 px-4 py-2.5">
  <div class="flex items-center gap-3">
    <StateTypeIcon type={issueState.type} />
    <span
      aria-hidden="true"
      class="size-2.5 rounded-full"
      style="background-color:{issueState.color}"
    ></span>

    {#if editing}
      <input
        bind:value={name}
        class="h-8 flex-1 rounded-md border border-border bg-card px-2 text-sm text-foreground focus:border-ring focus:outline-none"
        type="text"
      />
    {:else}
      <span class="flex-1 text-sm text-foreground">{issueState.name}</span>
    {/if}

    {#if issueState.isDefault}
      <span
        class="rounded-md bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground"
      >
        Default
      </span>
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
            aria-label="Move up"
            class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground disabled:opacity-30"
            disabled={busy || isFirst}
            onclick={() => run(onMoveUp)}
            type="button"
          >
            <ChevronUp class="size-3.5" />
          </button>
          <button
            aria-label="Move down"
            class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground disabled:opacity-30"
            disabled={busy || isLast}
            onclick={() => run(onMoveDown)}
            type="button"
          >
            <ChevronDown class="size-3.5" />
          </button>
          {#if !issueState.isDefault}
            <button
              aria-label="Set as default"
              class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground"
              onclick={() => run(onSetDefault)}
              type="button"
            >
              <Star class="size-3.5" />
            </button>
          {/if}
          <button
            class="h-7 rounded-md px-2 text-xs text-muted-foreground transition hover:bg-accent hover:text-foreground"
            onclick={startEdit}
            type="button"
          >
            Edit
          </button>
          <button
            aria-label="Delete state"
            class="grid size-7 place-items-center rounded-md text-muted-foreground transition hover:bg-destructive/10 hover:text-destructive"
            disabled={busy}
            onclick={() => run(onDelete)}
            type="button"
          >
            <Trash2 class="size-3.5" />
          </button>
        {/if}
      </div>
    {/if}
  </div>

  {#if editing}
    <div class="flex flex-wrap items-center gap-1.5 pl-9">
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
    <p class="pl-9 text-xs text-destructive" role="alert">{error}</p>
  {/if}
</div>
