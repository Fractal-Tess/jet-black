<script lang="ts">
import type { TicketRef } from "@workspace/shared/protocol";
import type { EstimateSystem } from "$lib/estimates";
import IssueComments from "./IssueComments.svelte";
import IssueDetailActions from "./IssueDetailActions.svelte";
import IssueDetailAttachments from "./IssueDetailAttachments.svelte";
import IssueDetailHeader from "./IssueDetailHeader.svelte";
import IssueDetailSubIssues from "./IssueDetailSubIssues.svelte";
import IssueEmptyState from "./IssueEmptyState.svelte";
import IssueExecutionPanel from "./IssueExecutionPanel.svelte";
import IssueProperties from "./IssueProperties.svelte";
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
} from "./types";

let {
  activities = [],
  attachments,
  comments,
  issue,
  labels,
  members,
  modules = [],
  estimateSystem,
  onAddAttachment,
  onArchiveIssue,
  onAddComment,
  onUpdateComment,
  onDeleteComment,
  onCreateLabel,
  onCreateModuleForIssue,
  onCreateSubIssue,
  onToggleLabel,
  onUpdateIssue,
  states,
  subIssues,
}: {
  activities?: IssueActivity[];
  attachments: IssueAttachment[];
  comments: IssueComment[];
  issue: Issue | null;
  labels: IssueLabel[];
  members: WorkspaceMember[];
  modules?: ProjectModuleRecord[];
  estimateSystem?: EstimateSystem;
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onAddComment: (
    body: string,
    parentCommentId?: IssueComment["_id"]
  ) => Promise<void>;
  onUpdateComment: (
    commentId: IssueComment["_id"],
    body: string
  ) => Promise<void>;
  onDeleteComment: (commentId: IssueComment["_id"]) => Promise<void>;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateModuleForIssue: (name: string) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
  states: IssueState[];
  subIssues: Issue[];
} = $props();

let showSubIssueForm = $state(false);
let showAttachForm = $state(false);

const executionTicket = $derived<Readonly<TicketRef> | null>(
  issue
    ? Object.freeze({
        control_plane_id: null,
        workspace_id: String(issue.workspaceId),
        project_id: String(issue.projectId),
        ticket_id: String(issue._id),
        identifier: issue.identifier,
        title: issue.title,
      })
    : null
);
</script>

{#if issue}
  <IssueDetailHeader {issue} {onUpdateIssue} />
  <IssueDetailActions
    bind:showSubIssueForm
    bind:showAttachForm
    {onArchiveIssue}
  />
  {#if executionTicket}
    <IssueExecutionPanel ticket={executionTicket} />
  {/if}
  <IssueDetailAttachments
    {attachments}
    bind:showAttachForm
    {onAddAttachment}
  />
  <div class="border-b border-border">
    <IssueProperties
      {estimateSystem}
      {issue}
      {labels}
      {members}
      {modules}
      {states}
      {onCreateLabel}
    {onCreateModuleForIssue}
      {onToggleLabel}
      {onUpdateIssue}
    />
  </div>
  <IssueDetailSubIssues
    {subIssues}
    bind:showSubIssueForm
    {onCreateSubIssue}
  />
  <IssueComments
    {activities}
    {comments}
    {issue}
    {onAddComment}
    {onDeleteComment}
    {onUpdateComment}
  />
{:else}
  <IssueEmptyState />
{/if}
