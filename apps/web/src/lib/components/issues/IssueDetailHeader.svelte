<script lang="ts">
import DescriptionEditor from "./DescriptionEditor.svelte";
import { useDescriptionUploader } from "./description-upload";
import type { Issue, UpdateIssueInput } from "./types";

let {
  issue,
  onUpdateIssue,
}: {
  issue: Issue;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
} = $props();

let editingTitle = $state(false);
let titleDraft = $state("");

const uploadDescriptionFile = useDescriptionUploader(() => issue.workspaceId);

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

function saveDescription(markdown: string) {
  if (markdown !== (issue.description ?? "")) {
    onUpdateIssue({ description: markdown });
  }
}
</script>

<div class="border-b border-border px-5 pt-5 pb-4">
  <p class="font-mono text-xs text-muted-foreground">{issue.identifier}</p>

  {#if editingTitle}
    <input
      aria-label="Issue title"
      bind:value={titleDraft}
      class="mt-2 w-full bg-transparent text-lg font-semibold text-foreground outline-none"
      onblur={saveTitle}
      onkeydown={(e) => {
        if (e.key === "Enter") saveTitle();
        if (e.key === "Escape") {
          editingTitle = false;
        }
      }}
    />
  {:else}
    <h2 class="mt-2">
      <button
      class="block w-full text-left text-lg font-semibold text-foreground transition-colors hover:text-primary"
        onclick={startEditTitle}
        type="button"
      >
        {issue.title}
      </button>
    </h2>
  {/if}

  <hr class="mt-3 border-border" />

  <DescriptionEditor
    ariaLabel="Description"
    class="mt-3 min-h-[60px] cursor-text"
    onBlur={saveDescription}
    placeholder="Click to add description"
    uploadFile={uploadDescriptionFile}
    value={issue.description ?? ""}
  />
</div>
