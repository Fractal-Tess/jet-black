<script lang="ts">
import { Button } from "@workspace/ui/components/button";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@workspace/ui/components/dialog";
import ArrowRight from "lucide-svelte/icons/arrow-right";
import ExternalLink from "lucide-svelte/icons/external-link";
import Maximize from "lucide-svelte/icons/maximize";
import PanelRight from "lucide-svelte/icons/panel-right";
import Shrink from "lucide-svelte/icons/shrink";
import { onMount } from "svelte";
import { cubicOut } from "svelte/easing";
import { fly } from "svelte/transition";
import { goto } from "$app/navigation";
import type {
  AddAttachmentInput,
  CreateLabelInput,
  Issue,
  IssueActivity,
  IssueAttachment,
  IssueComment,
  IssueLabel,
  IssueState,
  ProjectModuleRecord,
  UpdateIssueInput,
  WorkspaceMember,
} from "$lib/components/issues/types";
import type { EstimateSystem } from "$lib/estimates";
import { issueHref } from "$lib/routes";
import IssueDetail from "./IssueDetail.svelte";

let {
  issue,
  activities,
  attachments,
  comments,
  labels,
  members,
  modules = [],
  estimateSystem,
  onAddAttachment,
  onAddComment,
  onUpdateComment,
  onDeleteComment,
  onClose,
  onCreateLabel,
  onCreateModuleForIssue,
  onCreateSubIssue,
  onArchiveIssue,
  onToggleLabel,
  onUpdateIssue,
  states,
  subIssues,
  workspaceSlug,
}: {
  issue: Issue | null;
  activities: IssueActivity[];
  attachments: IssueAttachment[];
  comments: IssueComment[];
  labels: IssueLabel[];
  members: WorkspaceMember[];
  modules?: ProjectModuleRecord[];
  estimateSystem?: EstimateSystem;
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onAddComment: (
    body: string,
    parentCommentId?: IssueComment["_id"]
  ) => Promise<void>;
  onUpdateComment: (
    commentId: IssueComment["_id"],
    body: string
  ) => Promise<void>;
  onDeleteComment: (commentId: IssueComment["_id"]) => Promise<void>;
  onClose: () => void;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateModuleForIssue: (name: string) => Promise<void>;
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
  if (e.key === "Escape" && peekMode === "side") {
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

function dialogClasses(mode: PeekMode) {
  return mode === "full"
    ? "inset-4 h-auto w-auto max-w-none translate-x-0 translate-y-0 p-0 sm:max-w-none"
    : "h-[84vh] max-w-[84vw] p-0 sm:max-w-[84vw]";
}
</script>

{#snippet peekContent()}
  <div class="flex h-full flex-col bg-popover text-popover-foreground">
    <div class="flex shrink-0 items-center justify-between border-b border-border px-3 py-2">
      <div class="flex items-center gap-0.5">
        <Button
          aria-label="Close issue details"
          onclick={onClose}
          size="icon-sm"
          title="Close"
          variant="ghost"
        >
          <ArrowRight class="size-4" />
        </Button>
        <Button
          aria-label="Show issue full screen"
          class={peekMode === "full" ? "bg-accent text-accent-foreground" : ""}
          onclick={() => (peekMode = "full")}
          size="icon-sm"
          title="Full screen"
          variant="ghost"
        >
          <Maximize class="size-3.5" />
        </Button>
        {#if peekMode === "full"}
          <Button
            aria-label="Restore issue dialog"
            onclick={() => (peekMode = "modal")}
            size="icon-sm"
            title="Restore dialog"
            variant="ghost"
          >
            <Shrink class="size-3.5" />
          </Button>
        {/if}
        <Button
          aria-label="Dock issue details to the side"
          class={peekMode === "side" ? "bg-accent text-accent-foreground" : ""}
          onclick={() => (peekMode = "side")}
          size="icon-sm"
          title="Side peek"
          variant="ghost"
        >
          <PanelRight class="size-3.5" />
        </Button>
      </div>

      <div class="flex items-center gap-0.5">
        <Button
          aria-label="Open issue on its own page"
          onclick={openFullPage}
          size="icon-sm"
          title="Open in full page"
          variant="ghost"
        >
          <ExternalLink class="size-3.5" />
        </Button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto">
      <IssueDetail
        {activities}
        {attachments}
        {comments}
        {estimateSystem}
        {issue}
        {labels}
        {members}
        {modules}
        {onAddAttachment}
        {onArchiveIssue}
        {onAddComment}
        {onUpdateComment}
        {onDeleteComment}
        {onCreateLabel}
    {onCreateModuleForIssue}
        {onCreateSubIssue}
        {onToggleLabel}
        {onUpdateIssue}
        {states}
        {subIssues}
      />
    </div>
  </div>
{/snippet}

{#if issue && mounted}
  {#if peekMode === "side"}
    <aside
      aria-label="Issue details"
      class="fixed top-12 right-0 bottom-0 z-20 w-full border-l border-border shadow-xl sm:w-[480px]"
      transition:fly|global={{ duration: 250, easing: cubicOut, x: 480 }}
    >
      {@render peekContent()}
    </aside>
  {:else}
    <Dialog
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      open={true}
    >
      <DialogContent class={dialogClasses(peekMode)} showCloseButton={false}>
        <DialogTitle class="sr-only">
          {issue.identifier}: {issue.title}
        </DialogTitle>
        {@render peekContent()}
      </DialogContent>
    </Dialog>
  {/if}
{/if}
