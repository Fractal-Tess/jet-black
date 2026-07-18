<script lang="ts">
import Ellipsis from "lucide-svelte/icons/ellipsis";
import SquareStack from "lucide-svelte/icons/layers";
import Paperclip from "lucide-svelte/icons/paperclip";

let {
  showSubIssueForm = $bindable(),
  showAttachForm = $bindable(),
  onArchiveIssue,
}: {
  showSubIssueForm: boolean;
  showAttachForm: boolean;
  onArchiveIssue: () => Promise<void>;
} = $props();

let showMenu = $state(false);
let archiving = $state(false);

async function archiveIssue() {
  archiving = true;
  try {
    await onArchiveIssue();
  } finally {
    archiving = false;
    showMenu = false;
  }
}
</script>

<div class="flex items-center gap-2 border-b border-white/[0.06] px-5 py-3">
  <button
    class="flex items-center gap-1.5 rounded-md border border-white/10 px-2.5 py-1.5 text-xs text-zinc-400 transition hover:bg-white/[0.04] hover:text-zinc-200"
    onclick={() => (showSubIssueForm = !showSubIssueForm)}
    type="button"
  >
    <SquareStack class="size-3.5" />
    Add sub-work item
  </button>
  <button
    class="flex items-center gap-1.5 rounded-md border border-white/10 px-2.5 py-1.5 text-xs text-zinc-400 transition hover:bg-white/[0.04] hover:text-zinc-200"
    onclick={() => (showAttachForm = !showAttachForm)}
    type="button"
  >
    <Paperclip class="size-3.5" />
    Attach
  </button>

  <div class="ml-auto relative">
    <button
      class="grid size-7 place-items-center rounded-md text-zinc-500 transition hover:bg-white/[0.04] hover:text-zinc-300"
      onclick={() => (showMenu = !showMenu)}
      type="button"
    >
      <Ellipsis class="size-4" />
    </button>
    {#if showMenu}
      <div
        class="absolute right-0 top-8 z-10 w-36 rounded-lg border border-white/10 bg-[#1a1b1b] py-1 shadow-xl"
      >
        <button
          class="w-full px-3 py-2 text-left text-xs text-red-400 transition hover:bg-white/[0.04]"
          disabled={archiving}
          onclick={archiveIssue}
          type="button"
        >
          {archiving ? "Archiving\u2026" : "Archive issue"}
        </button>
      </div>
    {/if}
  </div>
</div>
