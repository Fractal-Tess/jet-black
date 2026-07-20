<script lang="ts">
import {
  groupIssues,
  type IssueDisplayOptions,
  type IssueGroup,
} from "./display-options";
import KanbanColumn from "./KanbanColumn.svelte";
import type {
  Issue,
  IssueState,
  UpdateIssueInput,
  WorkspaceMember,
} from "./types";

let {
  displayOptions,
  issues,
  members,
  onMoveIssue,
  onOpenCreateIssue,
  onQuickCreate,
  onReorderIssue,
  onSelect,
  onUpdateIssueFor,
  selectedIssueId,
  states,
}: {
  displayOptions: IssueDisplayOptions;
  issues: Issue[];
  members: WorkspaceMember[];
  onMoveIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onOpenCreateIssue?: (stateId: IssueState["_id"]) => void;
  onQuickCreate: (state: IssueState, title: string) => Promise<void>;
  onReorderIssue: (
    issue: Issue,
    stateId: IssueState["_id"],
    position: number
  ) => Promise<void>;
  onSelect: (issue: Issue) => void;
  onUpdateIssueFor: (issue: Issue, input: UpdateIssueInput) => Promise<void>;
  selectedIssueId?: string;
  states: IssueState[];
} = $props();

let draggingIssueId = $state<Issue["_id"] | null>(null);

const groups = $derived(groupIssues(issues, states, displayOptions));

const canReorder = $derived(
  displayOptions.groupBy === "state" && displayOptions.orderBy === "manual"
);

function positionForIssue(issue: Issue) {
  return issue.position ?? issue._creationTime;
}

function positionBetween(beforeIssue?: Issue, afterIssue?: Issue) {
  if (beforeIssue && afterIssue) {
    return (positionForIssue(beforeIssue) + positionForIssue(afterIssue)) / 2;
  }

  if (beforeIssue) {
    return positionForIssue(beforeIssue) + 1000;
  }

  if (afterIssue) {
    return positionForIssue(afterIssue) - 1000;
  }

  return Date.now();
}

function positionAtGroupEnd(group: IssueGroup, movedIssueId: Issue["_id"]) {
  const remaining = group.issues.filter((issue) => issue._id !== movedIssueId);

  return positionBetween(remaining.at(-1));
}

function handleDragStart(event: DragEvent, issue: Issue) {
  draggingIssueId = issue._id;
  event.dataTransfer?.setData("text/plain", issue._id);

  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
  }
}

function handleDragEnd() {
  draggingIssueId = null;
}

async function handleDrop(event: DragEvent, group: IssueGroup) {
  event.preventDefault();

  const issueId =
    (event.dataTransfer?.getData("text/plain") as Issue["_id"]) ??
    draggingIssueId;
  const issue = issues.find((candidate) => candidate._id === issueId);

  draggingIssueId = null;

  if (!issue) {
    return;
  }

  if (group.state) {
    await onMoveIssue(
      issue,
      group.state._id,
      positionAtGroupEnd(group, issue._id)
    );
    return;
  }

  if (group.priority && group.priority !== issue.priority) {
    await onUpdateIssueFor(issue, { priority: group.priority });
  }
}

async function handleDropOnCard(
  group: IssueGroup,
  index: number,
  edge: "after" | "before"
) {
  const issue = issues.find((candidate) => candidate._id === draggingIssueId);
  const target = group.issues[index];

  draggingIssueId = null;

  if (!(issue && target && group.state) || target._id === issue._id) {
    return;
  }

  // Compute neighbours as if the dragged issue was already removed so a
  // same-column move lands exactly where the indicator was shown.
  const remaining = group.issues.filter(
    (candidate) => candidate._id !== issue._id
  );
  const targetIndex = remaining.findIndex(
    (candidate) => candidate._id === target._id
  );
  const insertIndex = edge === "before" ? targetIndex : targetIndex + 1;
  const position = positionBetween(
    remaining[insertIndex - 1],
    remaining[insertIndex]
  );

  if (issue.stateId === group.state._id) {
    await onReorderIssue(issue, group.state._id, position);
    return;
  }

  await onMoveIssue(issue, group.state._id, position);
}

async function handleQuickCreate(group: IssueGroup, title: string) {
  if (!group.state) {
    return;
  }

  await onQuickCreate(group.state, title);
}

function handleOpenCreateIssue(group: IssueGroup) {
  if (!group.state) {
    return;
  }

  onOpenCreateIssue?.(group.state._id);
}
</script>

<div class="flex h-full min-h-0 gap-3 overflow-x-auto pb-4">
  {#each groups as group (group.id)}
    <KanbanColumn
      {canReorder}
      {draggingIssueId}
      {group}
      {members}
      onDragEnd={handleDragEnd}
      onDragStart={handleDragStart}
      onDrop={handleDrop}
      onDropOnCard={handleDropOnCard}
      onOpenCreateIssue={group.state && onOpenCreateIssue
        ? handleOpenCreateIssue
        : undefined}
      onQuickCreate={group.state ? handleQuickCreate : undefined}
      {onSelect}
      onUpdate={onUpdateIssueFor}
      properties={displayOptions.properties}
      {selectedIssueId}
    />
  {/each}
</div>
