<script lang="ts">
import type { AddAttachmentInput, IssueAttachment } from "./types";

let {
  attachments,
  onAddAttachment,
}: {
  attachments: IssueAttachment[];
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
} = $props();

let addingAttachment = $state(false);
let attachmentName = $state("");
let attachmentUrl = $state("");

async function addAttachment() {
  const name = attachmentName.trim();
  const url = attachmentUrl.trim();

  if (!(name && url)) {
    return;
  }

  addingAttachment = true;

  try {
    await onAddAttachment({ name, url });
    attachmentName = "";
    attachmentUrl = "";
  } finally {
    addingAttachment = false;
  }
}
</script>

<div class="mt-5 border-t border-border pt-4 px-4 pb-0">
  <div class="flex items-center justify-between gap-3">
    <h3 class="text-sm font-medium text-foreground">Attachments</h3>
    <span class="font-mono text-meta text-muted-foreground">
      {attachments.length} linked
    </span>
  </div>

  <div class="mt-3 space-y-2">
    {#each attachments as attachment (attachment._id)}
      <a
        class="block rounded-lg border border-border p-3 text-sm text-foreground transition-colors hover:border-input hover:bg-muted"
        href={attachment.url}
        rel="noreferrer"
        target="_blank"
      >
        <span class="block font-medium">{attachment.name}</span>
        <span class="mt-1 block truncate text-xs text-muted-foreground">
          {attachment.url}
        </span>
      </a>
    {:else}
      <p
      class="rounded-lg border border-dashed border-border p-3 text-xs text-muted-foreground"
      >
        Attach links to specs, mockups, or code references.
      </p>
    {/each}
  </div>

  <div class="mt-3 grid gap-2">
    <label class="block">
      <span class="sr-only">Attachment name</span>
      <input
        bind:value={attachmentName}
        class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="Attachment name"
      />
    </label>
    <label class="block">
      <span class="sr-only">Attachment URL</span>
      <input
        bind:value={attachmentUrl}
        class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/50"
        placeholder="https://example.com/spec"
        type="url"
      />
    </label>
    <button
      class="h-8 rounded-lg border border-border px-3 text-xs text-foreground transition-colors hover:bg-muted disabled:opacity-50"
      disabled={addingAttachment || !(attachmentName.trim() && attachmentUrl.trim())}
      onclick={addAttachment}
      type="button"
    >
      {addingAttachment ? "Adding\u2026" : "Add attachment"}
    </button>
  </div>
</div>
