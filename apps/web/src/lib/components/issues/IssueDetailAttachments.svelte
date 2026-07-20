<script lang="ts">
import Paperclip from "lucide-svelte/icons/paperclip";
import Plus from "lucide-svelte/icons/plus";
import X from "lucide-svelte/icons/x";
import type { AddAttachmentInput, IssueAttachment } from "./types";

let {
  attachments,
  showAttachForm = $bindable(),
  onAddAttachment,
}: {
  attachments: IssueAttachment[];
  showAttachForm: boolean;
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
} = $props();

let attachName = $state("");
let attachUrl = $state("");
let addingAttach = $state(false);

async function addAttachment() {
  const name = attachName.trim();
  const url = attachUrl.trim();
  if (!(name && url)) {
    return;
  }
  addingAttach = true;
  try {
    await onAddAttachment({ name, url });
    attachName = "";
    attachUrl = "";
    showAttachForm = false;
  } finally {
    addingAttach = false;
  }
}
</script>

<div class="border-b border-border px-5 py-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-medium text-foreground">Attachments</h3>
    <span class="text-xs text-muted-foreground">{attachments.length} linked</span>
  </div>

  {#if attachments.length > 0}
    <div class="mt-3 space-y-1.5">
      {#each attachments as attachment (attachment._id)}
        <a
        class="flex items-center gap-2 rounded-lg px-2 py-1.5 text-sm text-foreground transition-colors hover:bg-muted"
          href={attachment.url}
          rel="noreferrer"
          target="_blank"
        >
        <Paperclip class="size-3.5 shrink-0 text-muted-foreground" />
          <span class="truncate font-medium">{attachment.name}</span>
        <span class="ml-auto truncate text-xs text-muted-foreground"
            >{attachment.url}</span
          >
        </a>
      {/each}
    </div>
  {/if}

  {#if showAttachForm}
    <form
      class="mt-3 space-y-2"
      onsubmit={(e) => {
        e.preventDefault();
        addAttachment();
      }}
    >
      <input
        bind:value={attachName}
        class="h-8 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="Attachment name"
      />
      <input
        bind:value={attachUrl}
        class="h-8 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="https://example.com/spec"
        type="url"
      />
      <div class="flex gap-2">
        <button
        class="h-8 rounded-lg bg-primary px-3 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/80 disabled:opacity-50"
          disabled={addingAttach || !(attachName.trim() && attachUrl.trim())}
          type="submit"
        >
          {addingAttach ? "Adding\u2026" : "Add attachment"}
        </button>
        <button
        class="grid size-8 place-items-center rounded-lg border border-border text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          onclick={() => {
            showAttachForm = false;
            attachName = "";
            attachUrl = "";
          }}
          type="button"
        >
          <X class="size-3.5" />
        </button>
      </div>
    </form>
  {:else if attachments.length === 0}
    <button
      class="mt-3 flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
      onclick={() => (showAttachForm = true)}
      type="button"
    >
      <Plus class="size-3.5" />
      Add attachment
    </button>
  {/if}
</div>
