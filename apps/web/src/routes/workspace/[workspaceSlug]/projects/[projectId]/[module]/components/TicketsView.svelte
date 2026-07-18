<script lang="ts">
import type { IssueDisplayOptions } from "$lib/components/issues/display-options";
import IssueListView from "$lib/components/issues/IssueListView.svelte";
import IssuePeekOverview from "$lib/components/issues/IssuePeekOverview.svelte";
import KanbanBoard from "$lib/components/issues/KanbanBoard.svelte";
import type {
  AddAttachmentInput,
  CreateLabelInput,
  Issue,
  IssueAttachment,
  IssueComment,
  IssueLabel,
  IssueState,
  Project,
  UpdateIssueInput,
  WorkspaceMember,
} from "$lib/components/issues/types";

let {
  activeProject,
  attachments,
  comments,
  displayOptions,
  filteredIssues,
  issueView,
  labels,
  members,
  onAddAttachment,
  onAddComment,
  onCreateLabel,
  onCreateSubIssue,
  onArchiveIssue,
  onMoveIssue,
  onQuickCreateIssue,
  onReorderIssue,
  onToggleLabel,
  onUpdateIssue,
  onUpdateIssueFor,
  selectedIssueId,
  selectedSubIssues,
  states,
  workspaceSlug,
}: {
  activeProject: Project;
  attachments: IssueAttachment[];
  comments: IssueComment[];
  displayOptions: IssueDisplayOptions;
  filteredIssues: Issue[];
  issueView: "board" | "list";
  labels: IssueLabel[];
  members: WorkspaceMember[];
  onAddAttachment: (input: AddAttachmentInput) => Promise<void>;
  onAddComment: (body: string) => Promise<void>;
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onQuickCreateIssue: (state: IssueState, title: string) => Promise<void>;
  onReorderIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
  onUpdateIssueFor: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  selectedIssueId?: Issue["_id"];
  selectedSubIssues: Issue[];
  states: IssueState[];
  workspaceSlug: string;
} = $props();

let peekIssueId: string | null = $state(null);

const peekIssue = $derived(
  peekIssueId
    ? (filteredIssues.find((i) => i._id === peekIssueId) ?? null)
    : null
);

function openPeek(issue: Issue) {
  peekIssueId = issue._id;
}

function closePeek() {
  peekIssueId = null;
}

function handleSelect(issue: Issue) {
  if (peekIssueId === issue._id) {
    return;
  }
  peekIssueId = issue._id;
}
</script>

<div class="flex h-full flex-col">
  <div class="flex-1 overflow-hidden">
    {#if issueView === "board"}
      <KanbanBoard
        {displayOptions}
        issues={filteredIssues}
        {members}
        onMoveIssue={onMoveIssue}
        onQuickCreate={onQuickCreateIssue}
        onReorderIssue={onReorderIssue}
        onSelect={handleSelect}
        onUpdateIssueFor={onUpdateIssueFor}
        selectedIssueId={peekIssueId ?? selectedIssueId}
        {states}
      />
    {:else}
      <IssueListView
        {displayOptions}
        issues={filteredIssues}
        {members}
        onMoveIssue={onMoveIssue}
        onQuickCreate={onQuickCreateIssue}
        onSelect={handleSelect}
        onUpdateIssueFor={onUpdateIssueFor}
        selectedIssueId={peekIssueId ?? selectedIssueId}
        {states}
      />
    {/if}
  </div>

  <IssuePeekOverview
    {attachments}
    {comments}
    issue={peekIssue}
    {labels}
    {members}
    {onAddAttachment}
    {onAddComment}
    onClose={closePeek}
    {onCreateLabel}
    {onCreateSubIssue}
    {onArchiveIssue}
    {onToggleLabel}
    {onUpdateIssue}
    {states}
    subIssues={selectedSubIssues}
    {workspaceSlug}
  />
</div>
