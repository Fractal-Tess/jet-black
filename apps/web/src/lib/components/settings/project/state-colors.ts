export const STATE_COLOR_PALETTE = [
  "#6b7280",
  "#94a3b8",
  "#3b82f6",
  "#0ea5e9",
  "#8b5cf6",
  "#ec4899",
  "#f59e0b",
  "#eab308",
  "#22c55e",
  "#10b981",
  "#ef4444",
  "#f97316",
];

export type StateType =
  | "backlog"
  | "unstarted"
  | "started"
  | "completed"
  | "cancelled";

export const STATE_TYPE_ORDER: {
  type: StateType;
  label: string;
  description: string;
}[] = [
  { description: "Ideas and future work.", label: "Backlog", type: "backlog" },
  {
    description: "Ready to be worked on.",
    label: "Unstarted",
    type: "unstarted",
  },
  { description: "Actively in progress.", label: "Started", type: "started" },
  { description: "Finished work.", label: "Completed", type: "completed" },
  {
    description: "Abandoned or won't do.",
    label: "Cancelled",
    type: "cancelled",
  },
];
