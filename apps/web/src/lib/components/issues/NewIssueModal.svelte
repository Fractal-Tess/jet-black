<script lang="ts">
import { onMount } from "svelte";
import type { IssuePriority, Project } from "./types";

let {
  creating = false,
  isOpen = false,
  onClose,
  onCreate,
  project,
}: {
  creating?: boolean;
  isOpen?: boolean;
  onClose: () => void;
  onCreate: (input: {
    description?: string;
    priority: IssuePriority;
    title: string;
  }) => Promise<void>;
  project: Project;
} = $props();

let title = $state("");
let description = $state("");
let priority = $state<IssuePriority>("medium");
let error = $state("");
let mounted = $state(false);

onMount(() => {
  mounted = true;
  document.addEventListener("keydown", handleEscape);
  return () => document.removeEventListener("keydown", handleEscape);
});

function handleEscape(e: KeyboardEvent) {
  if (e.key === "Escape") {
    onClose();
  }
}

function reset() {
  title = "";
  description = "";
  priority = "medium";
  error = "";
}

async function submit() {
  error = "";

  if (!title.trim()) {
    error = "Title is required.";
    return;
  }

  try {
    await onCreate({
      description: description.trim() || undefined,
      priority,
      title,
    });
    reset();
    onClose();
  } catch (cause) {
    error =
      cause instanceof Error
        ? cause.message
        : "Could not create the work item.";
  }
}
</script>

{#if isOpen && mounted}
  <button
    aria-label="Close"
    class="fixed inset-0 z-50 bg-black/60"
    onclick={onClose}
    type="button"
  ></button>
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] pointer-events-none"
  >
    <div
      class="relative w-full max-w-lg rounded-xl border border-white/10 bg-[#151616] shadow-2xl pointer-events-auto"
      role="dialog"
      aria-modal="true"
      aria-label="Create work item"
    >
      <div class="flex items-center justify-between border-b border-white/10 px-6 py-4">
        <h2 class="text-base font-semibold text-zinc-100">Create work item</h2>
        <button
          aria-label="Close"
          class="grid size-7 place-items-center rounded-md text-zinc-500 transition hover:bg-white/5 hover:text-zinc-300"
          onclick={onClose}
          type="button"
        >
          ✕
        </button>
      </div>

      <form
        class="space-y-4 px-6 py-5"
        onsubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <label class="block" for="issue-title">
          <span class="mb-1.5 block text-sm text-zinc-300">
            Title
            <span class="text-red-400">*</span>
          </span>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            autofocus
            class="h-10 w-full rounded-md border border-zinc-700 bg-[#1a1b1b] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
            id="issue-title"
            placeholder="What needs to be done?"
            required
            bind:value={title}
          />
        </label>

        <label class="block" for="issue-description">
          <span class="mb-1.5 block text-sm text-zinc-300">Description</span>
          <textarea
            class="h-24 w-full resize-none rounded-md border border-zinc-700 bg-[#1a1b1b] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
            id="issue-description"
            placeholder="Add details…"
            bind:value={description}
          ></textarea>
        </label>

        <label class="block" for="issue-priority">
          <span class="mb-1.5 block text-sm text-zinc-300">Priority</span>
          <select
            class="h-10 w-full rounded-md border border-zinc-700 bg-[#1a1b1b] px-3 text-sm text-zinc-100 outline-none transition hover:border-zinc-600 focus:border-amber-400/70 focus:ring-2 focus:ring-amber-400/10"
            id="issue-priority"
            bind:value={priority}
          >
            <option value="none">None</option>
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
            <option value="urgent">Urgent</option>
          </select>
        </label>

        <div aria-live="polite" class="min-h-5">
          {#if error}
            <p class="text-sm text-red-400" role="alert">{error}</p>
          {/if}
        </div>

        <div class="flex items-center justify-end gap-3 border-t border-white/10 pt-4">
          <button
            class="h-9 rounded-md border border-white/10 px-4 text-xs text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200"
            onclick={onClose}
            type="button"
          >
            Cancel
          </button>
          <button
            class="flex h-9 items-center gap-2 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={creating || !title.trim()}
            type="submit"
          >
            {#if creating}
              <span
                aria-hidden="true"
                class="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent"
              ></span>
            {/if}
            {creating ? "Creating…" : "Create work item"}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
