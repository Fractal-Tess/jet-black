<script lang="ts">
import {
  compareIssues,
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

async function handleReorder(
  issue: Issue,
  group: IssueGroup,
  direction: "down" | "up",
  index: number
) {
  if (!group.state) {
    return;
  }

  const position =
    direction === "up"
      ? positionBetween(group.issues[index - 2], group.issues[index - 1])
      : positionBetween(group.issues[index + 1], group.issues[index + 2]);

  await onReorderIssue(issue, group.state._id, position);
}

async function handleMoveToState(issue: Issue, stateId: IssueState["_id"]) {
  const targetIssues = issues
    .filter(
      (candidate) =>
        candidate.stateId === stateId && candidate._id !== issue._id
    )
    .toSorted((left, right) => compareIssues(left, right, "manual"));

  await onMoveIssue(issue, stateId, positionBetween(targetIssues.at(-1)));
}

async function handleQuickCreate(group: IssueGroup, title: string) {
  if (!group.state) {
    return;
  }

  await onQuickCreate(group.state, title);
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
      onMoveToState={handleMoveToState}
      onQuickCreate={group.state ? handleQuickCreate : undefined}
      onReorder={handleReorder}
      {onSelect}
      onUpdate={onUpdateIssueFor}
      properties={displayOptions.properties}
      {selectedIssueId}
      {states}
    />
  {/each}
</div>
