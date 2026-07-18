import type { IssuePriority } from "./types";

export const priorityOptions: {
  value: IssuePriority;
  label: string;
  color: string;
}[] = [
  { value: "urgent", label: "Urgent", color: "#ef4444" },
  { value: "high", label: "High", color: "#f97316" },
  { value: "medium", label: "Medium", color: "#f59e0b" },
  { value: "low", label: "Low", color: "#22c55e" },
  { value: "none", label: "None", color: "#71717a" },
];
