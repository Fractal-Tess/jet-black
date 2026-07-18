<script lang="ts">
import ArrowRight from "lucide-svelte/icons/arrow-right";
import ExternalLink from "lucide-svelte/icons/external-link";
import Maximize from "lucide-svelte/icons/maximize";
import PanelRight from "lucide-svelte/icons/panel-right";
import Shrink from "lucide-svelte/icons/shrink";
import { onMount } from "svelte";
import { goto } from "$app/navigation";
import type {
  AddAttachmentInput,
  CreateLabelInput,
  Issue,
  IssueAttachment,
  IssueComment,
  IssueLabel,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "$lib/components/issues/types";
import { issueHref } from "$lib/routes";
import IssueDetail from "./IssueDetail.svelte";

let {
  issue,
  attachments,
  comments,
  labels,
  members,
  onAddAttachment,
  onAddComment,
  onClose,
  onCreateLabel,
  onCreateSubIssue,
  onArchiveIssue,
  onToggleLabel,
  onUpdateIssue,
  states,
  subIssues,
  workspaceSlug,
}: {
  issue: Issue | null;
  attachments: IssueAttachment[];
  comments: IssueComment[];
  labels: IssueLabel[];
  members: WorkspaceMember[];
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onAddComment: (body: string) => Promise<void>;
  onClose: () => void;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
  states: IssueState[];
  subIssues: Issue[];
  workspaceSlug: string;
} = $props();

type PeekMode = "side" | "modal" | "full";

let peekMode: PeekMode = $state("side");
let mounted = $state(false);

onMount(() => {
  mounted = true;
  document.addEventListener("keydown", handleKeydown);
  return () => document.removeEventListener("keydown", handleKeydown);
});

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    onClose();
  }
}

function openFullPage() {
  if (!issue) {
    return;
  }
  goto(
    issueHref({
      issueId: issue._id,
      projectId: issue.projectId,
      workspaceSlug,
    })
  );
}

const modeClasses = $derived({
  side: "top-0 right-0 bottom-0 w-[480px] border-l border-white/10 translate-x-0",
  modal:
    "top-[8%] left-[8%] w-[84%] h-[84%] rounded-xl border border-white/10 shadow-2xl",
  full: "inset-4 rounded-xl border border-white/10 shadow-2xl",
});
</script>

{#if issue && mounted}
  {#if peekMode === "modal" || peekMode === "full"}
    <button
      aria-label="Close"
      class="fixed inset-0 z-40 bg-black/60"
      onclick={onClose}
      type="button"
    ></button>
  {/if}

  <div
    class="fixed z-50 flex flex-col bg-[#111212] text-zinc-200 transition-all duration-300 {modeClasses[peekMode]}"
  >
    <div class="flex shrink-0 items-center justify-between border-b border-white/10 px-3 py-2">
      <div class="flex items-center gap-0.5">
        <button
          class="grid size-7 place-items-center rounded text-zinc-500 transition hover:bg-white/5 hover:text-zinc-300"
          onclick={onClose}
          title="Close"
          type="button"
        >
          <ArrowRight class="size-4" />
        </button>
        <button
          class="grid size-7 place-items-center rounded transition {peekMode === 'full'
            ? 'bg-white/10 text-zinc-100'
            : 'text-zinc-500 hover:bg-white/5 hover:text-zinc-300'}"
          onclick={() => (peekMode = "full")}
          title="Full screen"
          type="button"
        >
          <Maximize class="size-3.5" />
        </button>
        <button
          class="grid size-7 place-items-center rounded transition {peekMode === 'side'
            ? 'bg-white/10 text-zinc-100'
            : 'text-zinc-500 hover:bg-white/5 hover:text-zinc-300'}"
          onclick={() => (peekMode = "side")}
          title="Side peek"
          type="button"
        >
          <PanelRight class="size-3.5" />
        </button>
      </div>

      <div class="flex items-center gap-0.5">
        <button
          class="grid size-7 place-items-center rounded text-zinc-500 transition hover:bg-white/5 hover:text-zinc-300"
          onclick={openFullPage}
          title="Open in full page"
          type="button"
        >
          <ExternalLink class="size-3.5" />
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto">
      <IssueDetail
        {attachments}
        {comments}
        {issue}
        {labels}
        {members}
        {onAddAttachment}
        {onArchiveIssue}
        {onAddComment}
        {onCreateLabel}
        {onCreateSubIssue}
        {onToggleLabel}
        {onUpdateIssue}
        {states}
        {subIssues}
      />
    </div>
  </div>
{/if}
