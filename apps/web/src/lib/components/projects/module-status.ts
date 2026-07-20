import type { ProjectModuleRecord } from "$lib/components/issues/types";

export type ModuleStatus = ProjectModuleRecord["status"];

// Mirrors Plane's MODULE_STATUS constants (packages/constants/src/module.ts).
export const MODULE_STATUSES: {
  label: string;
  value: ModuleStatus;
}[] = [
  { label: "Backlog", value: "backlog" },
  { label: "Planned", value: "planned" },
  { label: "In progress", value: "in_progress" },
  { label: "Paused", value: "paused" },
  { label: "Completed", value: "completed" },
  { label: "Cancelled", value: "cancelled" },
];

export function moduleStatusLabel(status: ModuleStatus): string {
  return (
    MODULE_STATUSES.find((s) => s.value === status)?.label ??
    status.charAt(0).toUpperCase() + status.slice(1)
  );
}

export function moduleStatusBadgeClass(status: ModuleStatus): string {
  switch (status) {
    case "planned":
      return "bg-info/15 text-info";
    case "in_progress":
      return "bg-warning/15 text-warning";
    case "completed":
      return "bg-success/15 text-success";
    case "cancelled":
      return "bg-destructive/15 text-destructive";
    case "paused":
      return "bg-secondary text-secondary-foreground";
    default:
      return "bg-muted text-muted-foreground";
  }
}

export function moduleStatusDotClass(status: ModuleStatus): string {
  switch (status) {
    case "planned":
      return "bg-info";
    case "in_progress":
      return "bg-warning";
    case "completed":
      return "bg-success";
    case "cancelled":
      return "bg-destructive";
    default:
      return "bg-muted-foreground";
  }
}
