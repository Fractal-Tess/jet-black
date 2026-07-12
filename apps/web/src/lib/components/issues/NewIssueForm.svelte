<script lang="ts">
import type { IssuePriority, Project } from "./types";

let {
  creating = false,
  onCreate,
  project,
}: {
  creating?: boolean;
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
    title = "";
    description = "";
    priority = "medium";
  } catch (submissionError) {
    error =
      submissionError instanceof Error
        ? submissionError.message
        : "Could not create issue.";
  }
}
</script>

<section
  class="rounded-xl border border-white/10 bg-[#151616] shadow-2xl shadow-black/20"
  id="new-issue"
>
  <div class="border-b border-white/[0.06] px-4 py-3">
    <p class="text-xs font-medium uppercase tracking-[0.18em] text-amber-400">
      New work item
    </p>
    <h2 class="mt-1 text-lg font-semibold text-zinc-100">{project.name}</h2>
  </div>

  <form class="space-y-3 p-4" onsubmit={(event) => event.preventDefault()}>
    <label class="block">
      <span class="sr-only">Issue title</span>
      <input
        bind:value={title}
        class="h-10 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60 focus:ring-2 focus:ring-amber-400/10"
        placeholder="Add a title, e.g. Build realtime issue updates"
      />
    </label>

    <label class="block">
      <span class="sr-only">Issue description</span>
      <textarea
        bind:value={description}
        class="min-h-20 w-full resize-y rounded-md border border-white/10 bg-[#0f1010] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60 focus:ring-2 focus:ring-amber-400/10"
        placeholder="Add useful context. Keep it short for now."
      ></textarea>
    </label>

    <div class="flex flex-col gap-3 sm:flex-row sm:items-center">
      <label class="flex items-center gap-2 text-xs text-zinc-500">
        Priority
        <select
          bind:value={priority}
          class="h-9 rounded-md border border-white/10 bg-[#0f1010] px-2 text-xs text-zinc-200 outline-none focus:border-amber-400/60"
        >
          <option value="none">None</option>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
          <option value="urgent">Urgent</option>
        </select>
      </label>

      <button
        class="ml-auto h-9 rounded-md bg-amber-400 px-4 text-xs font-semibold text-black transition hover:bg-amber-300 disabled:cursor-not-allowed disabled:opacity-50"
        disabled={creating || !title.trim()}
        onclick={submit}
        type="button"
      >
        {creating ? "Creating…" : "Create issue"}
      </button>
    </div>

    {#if error}
      <p class="text-xs text-red-300">{error}</p>
    {/if}
  </form>
</section>
