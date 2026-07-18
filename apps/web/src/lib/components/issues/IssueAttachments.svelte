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

<div class="mt-5 border-t border-white/[0.06] pt-4 px-4 pb-0">
  <div class="flex items-center justify-between gap-3">
    <h3 class="text-sm font-medium text-zinc-300">Attachments</h3>
    <span class="font-mono text-[10px] text-zinc-700">
      {attachments.length} linked
    </span>
  </div>

  <div class="mt-3 space-y-2">
    {#each attachments as attachment (attachment._id)}
      <a
        class="block rounded-md border border-white/[0.06] p-3 text-sm text-zinc-300 transition hover:border-white/15 hover:bg-white/[0.03]"
        href={attachment.url}
        rel="noreferrer"
        target="_blank"
      >
        <span class="block font-medium">{attachment.name}</span>
        <span class="mt-1 block truncate text-xs text-zinc-600">
          {attachment.url}
        </span>
      </a>
    {:else}
      <p
        class="rounded-md border border-dashed border-white/10 p-3 text-xs text-zinc-600"
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
        class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
        placeholder="Attachment name"
      />
    </label>
    <label class="block">
      <span class="sr-only">Attachment URL</span>
      <input
        bind:value={attachmentUrl}
        class="h-9 w-full rounded-md border border-white/10 bg-[#0f1010] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-400/60"
        placeholder="https://example.com/spec"
        type="url"
      />
    </label>
    <button
      class="h-8 rounded-md border border-white/10 px-3 text-xs text-zinc-300 transition hover:bg-white/[0.04] disabled:opacity-50"
      disabled={addingAttachment || !(attachmentName.trim() && attachmentUrl.trim())}
      onclick={addAttachment}
      type="button"
    >
      {addingAttachment ? "Adding\u2026" : "Add attachment"}
    </button>
  </div>
</div>
