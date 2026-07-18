<script lang="ts">
import type { Issue, UpdateIssueInput } from "./types";

let {
  issue,
  onUpdateIssue,
}: {
  issue: Issue;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
} = $props();

let editingTitle = $state(false);
let editingDescription = $state(false);
let titleDraft = $state("");
let descriptionDraft = $state("");

function startEditTitle() {
  titleDraft = issue.title;
  editingTitle = true;
}

function saveTitle() {
  const trimmed = titleDraft.trim();
  if (trimmed && trimmed !== issue.title) {
    onUpdateIssue({ title: trimmed });
  }
  editingTitle = false;
}

function startEditDescription() {
  descriptionDraft = issue.description ?? "";
  editingDescription = true;
}

function saveDescription() {
  if (descriptionDraft !== (issue.description ?? "")) {
    onUpdateIssue({ description: descriptionDraft });
  }
  editingDescription = false;
}
</script>

<div class="border-b border-white/[0.06] px-5 pt-5 pb-4">
  <p class="font-mono text-xs text-zinc-500">{issue.identifier}</p>

  {#if editingTitle}
    <input
      bind:value={titleDraft}
      class="mt-2 w-full bg-transparent text-lg font-semibold text-zinc-100 outline-none"
      onblur={saveTitle}
      onkeydown={(e) => {
        if (e.key === "Enter") saveTitle();
        if (e.key === "Escape") {
          editingTitle = false;
        }
      }}
    />
  {:else}
    <button
      class="mt-2 block w-full text-left text-lg font-semibold text-zinc-100 transition hover:text-white"
      onclick={startEditTitle}
      type="button"
    >
      {issue.title}
    </button>
  {/if}

  {#if editingDescription}
    <textarea
      bind:value={descriptionDraft}
      class="mt-3 min-h-[60px] w-full resize-y bg-transparent text-sm text-zinc-300 outline-none placeholder:text-zinc-600"
      onblur={saveDescription}
      placeholder="Add a description..."
    ></textarea>
  {:else}
    <button
      class="mt-3 block w-full text-left text-sm transition {issue.description ? 'text-zinc-400' : 'text-zinc-600'} hover:text-zinc-300"
      onclick={startEditDescription}
      type="button"
    >
      {issue.description || "Click to add a description"}
    </button>
  {/if}
</div>
