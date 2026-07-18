<script lang="ts">
import IssueComments from "./IssueComments.svelte";
import IssueDetailActions from "./IssueDetailActions.svelte";
import IssueDetailAttachments from "./IssueDetailAttachments.svelte";
import IssueDetailHeader from "./IssueDetailHeader.svelte";
import IssueDetailSubIssues from "./IssueDetailSubIssues.svelte";
import IssueEmptyState from "./IssueEmptyState.svelte";
import IssueProperties from "./IssueProperties.svelte";
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
} from "./types";

let {
  attachments,
  comments,
  issue,
  labels,
  members,
  onAddAttachment,
  onArchiveIssue,
  onAddComment,
  onCreateLabel,
  onCreateSubIssue,
  onToggleLabel,
  onUpdateIssue,
  states,
  subIssues,
}: {
  attachments: IssueAttachment[];
  comments: IssueComment[];
  issue: Issue | null;
  labels: IssueLabel[];
  members: WorkspaceMember[];
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onAddComment: (body: string) => Promise<void>;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
  states: IssueState[];
  subIssues: Issue[];
} = $props();

let showSubIssueForm = $state(false);
let showAttachForm = $state(false);
</script>

{#if issue}
  <IssueDetailHeader {issue} {onUpdateIssue} />
  <IssueDetailActions
    bind:showSubIssueForm
    bind:showAttachForm
    {onArchiveIssue}
  />
  <div class="border-b border-white/[0.06]">
    <IssueProperties
      {issue}
      {labels}
      {members}
      {states}
      {onCreateLabel}
      {onToggleLabel}
      {onUpdateIssue}
    />
  </div>
  <IssueDetailSubIssues
    {subIssues}
    bind:showSubIssueForm
    {onCreateSubIssue}
  />
  <IssueDetailAttachments
    {attachments}
    bind:showAttachForm
    {onAddAttachment}
  />
  <IssueComments {issue} {comments} {onAddComment} />
{:else}
  <IssueEmptyState />
{/if}
