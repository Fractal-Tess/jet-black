import type { Issue, IssuePriority, IssueState } from "./types";

export type IssueLayout = "list" | "board";

export type IssueGroupBy = "state" | "priority";

export type IssueOrderBy = "manual" | "priority" | "created" | "updated";

export type IssueDisplayProperty =
  | "key"
  | "state"
  | "priority"
  | "assignee"
  | "labels"
  | "dueDate";

export type IssueDisplayOptions = {
  groupBy: IssueGroupBy;
  orderBy: IssueOrderBy;
  properties: Record<IssueDisplayProperty, boolean>;
};

export const DISPLAY_PROPERTY_LABELS: Record<IssueDisplayProperty, string> = {
  assignee: "Assignee",
  dueDate: "Due date",
  key: "ID",
  labels: "Labels",
  priority: "Priority",
  state: "State",
};

export const GROUP_BY_OPTIONS: { label: string; value: IssueGroupBy }[] = [
  { label: "States", value: "state" },
  { label: "Priority", value: "priority" },
];

export const ORDER_BY_OPTIONS: { label: string; value: IssueOrderBy }[] = [
  { label: "Manual", value: "manual" },
  { label: "Priority", value: "priority" },
  { label: "Created date", value: "created" },
  { label: "Last updated", value: "updated" },
];

export const PRIORITY_ORDER: IssuePriority[] = [
  "urgent",
  "high",
  "medium",
  "low",
  "none",
];

export const PRIORITY_LABELS: Record<IssuePriority, string> = {
  high: "High",
  low: "Low",
  medium: "Medium",
  none: "None",
  urgent: "Urgent",
};

export function defaultDisplayOptions(): IssueDisplayOptions {
  return {
    groupBy: "state",
    orderBy: "manual",
    properties: {
      assignee: true,
      dueDate: true,
      key: true,
      labels: true,
      priority: true,
      state: true,
    },
  };
}

export type IssueGroup = {
  color?: string;
  id: string;
  issues: Issue[];
  name: string;
  priority?: IssuePriority;
  state?: IssueState;
};

const PRIORITY_WEIGHT: Record<IssuePriority, number> = {
  high: 3,
  low: 1,
  medium: 2,
  none: 0,
  urgent: 4,
};

function manualPosition(issue: Issue) {
  return issue.position ?? issue._creationTime;
}

export function compareIssues(
  left: Issue,
  right: Issue,
  orderBy: IssueOrderBy
): number {
  if (orderBy === "priority") {
    return PRIORITY_WEIGHT[right.priority] - PRIORITY_WEIGHT[left.priority];
  }

  if (orderBy === "created") {
    return right._creationTime - left._creationTime;
  }

  if (orderBy === "updated") {
    return right.updatedAt - left.updatedAt;
  }

  const positionDelta = manualPosition(left) - manualPosition(right);

  if (positionDelta !== 0) {
    return positionDelta;
  }

  return left.identifier.localeCompare(right.identifier);
}

export function groupIssues(
  issues: Issue[],
  states: IssueState[],
  options: IssueDisplayOptions
): IssueGroup[] {
  const sorted = issues.toSorted((left, right) =>
    compareIssues(left, right, options.orderBy)
  );

  if (options.groupBy === "priority") {
    return PRIORITY_ORDER.map((priority) => ({
      id: priority,
      issues: sorted.filter((issue) => issue.priority === priority),
      name: PRIORITY_LABELS[priority],
      priority,
    }));
  }

  return states
    .toSorted((left, right) => left.position - right.position)
    .map((state) => ({
      color: state.color,
      id: state._id,
      issues: sorted.filter((issue) => issue.stateId === state._id),
      name: state.name,
      state,
    }));
}
