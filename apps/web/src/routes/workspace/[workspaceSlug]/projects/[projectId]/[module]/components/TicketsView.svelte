<script lang="ts">
import { page } from "$app/state";
import type { IssueDisplayOptions } from "$lib/components/issues/display-options";
import IssueListView from "$lib/components/issues/IssueListView.svelte";
import IssuePeekOverview from "$lib/components/issues/IssuePeekOverview.svelte";
import KanbanBoard from "$lib/components/issues/KanbanBoard.svelte";
import type {
  AddAttachmentInput,
  CreateLabelInput,
  Issue,
  IssueActivity,
  IssueAttachment,
  IssueComment,
  IssueLabel,
  IssueState,
  Project,
  ProjectModuleRecord,
  UpdateIssueInput,
  WorkspaceMember,
} from "$lib/components/issues/types";

let {
  activeProject,
  activities,
  attachments,
  comments,
  displayOptions,
  filteredIssues,
  issueView,
  labels,
  members,
  modules,
  onAddAttachment,
  onAddComment,
  onUpdateComment,
  onDeleteComment,
  onCreateLabel,
  onCreateModuleForIssue,
  onCreateSubIssue,
  onArchiveIssue,
  onMoveIssue,
  onOpenCreateIssue,
  onQuickCreateIssue,
  onReorderIssue,
  onSelectIssue,
  onToggleLabel,
  onUpdateIssue,
  onUpdateIssueFor,
  selectedIssueId,
  selectedSubIssues,
  states,
  workspaceSlug,
}: {
  activeProject: Project;
  activities: IssueActivity[];
  attachments: IssueAttachment[];
  comments: IssueComment[];
  displayOptions: IssueDisplayOptions;
  filteredIssues: Issue[];
  issueView: "board" | "list";
  labels: IssueLabel[];
  members: WorkspaceMember[];
  modules: ProjectModuleRecord[];
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
  onCreateLabel: (input: CreateLabelInput) => Promise<void>;
  onCreateModuleForIssue: (name: string) => Promise<void>;
  onCreateSubIssue: (title: string) => Promise<void>;
  onArchiveIssue: () => Promise<void>;
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onOpenCreateIssue?: (stateId: IssueState["_id"]) => void;
  onQuickCreateIssue: (state: IssueState, title: string) => Promise<void>;
  onReorderIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onSelectIssue: (issueId: Issue["_id"]) => void;
  onToggleLabel: (labelId: IssueLabel["_id"]) => Promise<void>;
  onUpdateIssue: (input: UpdateIssueInput) => Promise<void>;
  onUpdateIssueFor: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  selectedIssueId?: Issue["_id"];
  selectedSubIssues: Issue[];
  states: IssueState[];
  workspaceSlug: string;
} = $props();

let peekIssueId: string | null = $state(null);

// Open the peek panel when the route points at an issue (deep links,
// post-create navigation). Local close still wins until the route changes.
$effect(() => {
  const routeIssueId = page.params.issueId;

  if (routeIssueId) {
    peekIssueId = routeIssueId;
  }
});

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

function handleBackgroundClick(event: MouseEvent) {
  const target = event.target as HTMLElement;

  // Only clicks on empty board space should dismiss the peek — cards,
  // chips, quick-add forms, and other controls handle their own clicks.
  if (target.closest("article, button, input, a, form")) {
    return;
  }

  closePeek();
}

function handleSelect(issue: Issue) {
  // Keep the workspace-level selection in sync so issue-scoped actions
  // (archive, comments, attachments) target the issue shown in the peek.
  onSelectIssue(issue._id);
  if (peekIssueId === issue._id) {
    return;
  }
  peekIssueId = issue._id;
}
</script>

<div class="flex h-full flex-col">
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="flex-1 overflow-hidden" onclick={handleBackgroundClick}>
    {#if issueView === "board"}
      <KanbanBoard
        {displayOptions}
        issues={filteredIssues}
        {members}
        onMoveIssue={onMoveIssue}
        {onOpenCreateIssue}
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
        onQuickCreate={onQuickCreateIssue}
        onSelect={handleSelect}
        onUpdateIssueFor={onUpdateIssueFor}
        selectedIssueId={peekIssueId ?? selectedIssueId}
        {states}
      />
    {/if}
  </div>

  <IssuePeekOverview
    {activities}
    {attachments}
    {comments}
    estimateSystem={activeProject.estimateSystem}
    issue={peekIssue}
    {labels}
    {members}
    {modules}
    {onAddAttachment}
    {onAddComment}
    {onUpdateComment}
    {onDeleteComment}
    onClose={closePeek}
    {onCreateLabel}
    {onCreateModuleForIssue}
    {onCreateSubIssue}
    {onArchiveIssue}
    {onToggleLabel}
    {onUpdateIssue}
    {states}
    subIssues={selectedSubIssues}
    {workspaceSlug}
  />
</div>
