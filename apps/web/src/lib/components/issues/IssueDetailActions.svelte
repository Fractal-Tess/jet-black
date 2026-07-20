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

<div class="flex items-center gap-2 border-b border-border px-5 py-3">
  <button
    class="flex items-center gap-1.5 rounded-lg border border-border px-2.5 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
    onclick={() => (showSubIssueForm = !showSubIssueForm)}
    type="button"
  >
    <SquareStack class="size-3.5" />
    Add sub-work item
  </button>
  <button
    class="flex items-center gap-1.5 rounded-lg border border-border px-2.5 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
    onclick={() => (showAttachForm = !showAttachForm)}
    type="button"
  >
    <Paperclip class="size-3.5" />
    Attach
  </button>

  <div class="ml-auto relative">
    <button
      aria-label="Issue actions"
      class="grid size-7 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      onclick={() => (showMenu = !showMenu)}
      type="button"
    >
      <Ellipsis class="size-4" />
    </button>
    {#if showMenu}
      <div
      class="absolute right-0 top-8 z-10 w-36 rounded-lg border border-border bg-popover py-1 text-popover-foreground shadow-md"
      >
        <button
        class="w-full px-3 py-2 text-left text-xs text-destructive transition-colors hover:bg-muted"
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
