import type { IssuePriority } from "./types";

export const priorityOptions: {
  value: IssuePriority;
  label: string;
}[] = [
  { value: "urgent", label: "Urgent" },
  { value: "high", label: "High" },
  { value: "medium", label: "Medium" },
  { value: "low", label: "Low" },
  { value: "none", label: "None" },
];
