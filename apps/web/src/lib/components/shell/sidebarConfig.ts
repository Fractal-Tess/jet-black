import CheckSquare from "lucide-svelte/icons/check-square";
import FileText from "lucide-svelte/icons/file-text";
import Inbox from "lucide-svelte/icons/inbox";
import LayoutGrid from "lucide-svelte/icons/layout-grid";
import Package from "lucide-svelte/icons/package";
import Repeat from "lucide-svelte/icons/repeat";
import type { ProjectModule } from "$lib/routes";

export const moduleLinks: { label: string; module: ProjectModule }[] = [
  { label: "Tickets", module: "tickets" },
  { label: "Intake", module: "intake" },
  { label: "Sprints", module: "sprints" },
  { label: "Modules", module: "modules" },
  { label: "Views", module: "views" },
  { label: "Pages", module: "pages" },
];

export const moduleIcons: Record<ProjectModule, typeof CheckSquare> = {
  tickets: CheckSquare,
  issues: CheckSquare,
  intake: Inbox,
  sprints: Repeat,
  modules: Package,
  views: LayoutGrid,
  pages: FileText,
} as const;
