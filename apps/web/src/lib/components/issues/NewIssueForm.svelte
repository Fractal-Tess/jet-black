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
  class="rounded-xl border border-border bg-card text-card-foreground shadow-sm"
  id="new-issue"
>
  <div class="border-b border-border px-4 py-3">
    <p class="text-xs font-medium uppercase tracking-wider text-primary">
      New work item
    </p>
    <h2 class="mt-1 text-lg font-semibold text-card-foreground">{project.name}</h2>
  </div>

  <form class="space-y-3 p-4" onsubmit={(event) => event.preventDefault()}>
    <label class="block">
      <span class="sr-only">Issue title</span>
      <input
        bind:value={title}
        class="h-10 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="Add a title, e.g. Build realtime issue updates"
      />
    </label>

    <label class="block">
      <span class="sr-only">Issue description</span>
      <textarea
        bind:value={description}
        class="min-h-20 w-full resize-y rounded-lg border border-input bg-background px-3 py-2 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="Add useful context. Keep it short for now."
      ></textarea>
    </label>

    <div class="flex flex-col gap-3 sm:flex-row sm:items-center">
      <label class="flex items-center gap-2 text-xs text-muted-foreground">
        Priority
        <select
          bind:value={priority}
          class="h-9 rounded-lg border border-input bg-background px-2 text-xs text-foreground outline-none focus:border-ring"
        >
          <option value="none">None</option>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
          <option value="urgent">Urgent</option>
        </select>
      </label>

      <button
        class="ml-auto h-9 rounded-lg bg-primary px-4 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/80 disabled:cursor-not-allowed disabled:opacity-50"
        disabled={creating || !title.trim()}
        onclick={submit}
        type="button"
      >
        {creating ? "Creating…" : "Create issue"}
      </button>
    </div>

    {#if error}
      <p class="text-xs text-destructive">{error}</p>
    {/if}
  </form>
</section>
