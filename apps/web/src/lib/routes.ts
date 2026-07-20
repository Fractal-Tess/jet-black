import type { Project, ViewerData } from "$lib/components/issues/types";

export type WorkspacePage = "analytics" | "home";

export type ProjectModule =
  | "intake"
  | "issues"
  | "modules"
  | "pages"
  | "sprints"
  | "tickets"
  | "views";

const DEFAULT_MODULE: ProjectModule = "tickets";
const PROJECT_MODULES = [
  "intake",
  "issues",
  "modules",
  "pages",
  "sprints",
  "tickets",
  "views",
] as const;

type WorkspaceForHref = NonNullable<ViewerData["activeWorkspace"]>;

export function workspaceHref(
  workspace: Pick<WorkspaceForHref, "slug"> | string
) {
  const slug = typeof workspace === "string" ? workspace : workspace.slug;
  return `/workspace/${slug}`;
}

export function workspaceProjectsHref(
  workspace: Pick<WorkspaceForHref, "slug"> | string
) {
  return `${workspaceHref(workspace)}/projects`;
}

export function workspaceAnalyticsHref(
  workspace: Pick<WorkspaceForHref, "slug"> | string
) {
  return `${workspaceHref(workspace)}/analytics`;
}

export function projectModuleHref(input: {
  module?: ProjectModule;
  projectId: Project["_id"] | string;
  workspaceSlug: string;
}) {
  return `${workspaceProjectsHref(input.workspaceSlug)}/${input.projectId}/${
    input.module ?? DEFAULT_MODULE
  }`;
}

export function issueHref(input: {
  issueId: string;
  projectId: Project["_id"] | string;
  workspaceSlug: string;
}) {
  return `${projectModuleHref({
    module: "issues",
    projectId: input.projectId,
    workspaceSlug: input.workspaceSlug,
  })}/${input.issueId}`;
}

export function projectModuleDetailHref(input: {
  moduleId: string;
  projectId: Project["_id"] | string;
  workspaceSlug: string;
}) {
  return `${workspaceProjectsHref(input.workspaceSlug)}/${input.projectId}/modules/${input.moduleId}`;
}

export function normalizeProjectModule(value?: string): ProjectModule {
  if (isProjectModule(value)) {
    return value;
  }

  return DEFAULT_MODULE;
}

export function isProjectModule(
  value: string | undefined
): value is ProjectModule {
  return (
    typeof value === "string" &&
    PROJECT_MODULES.includes(value as ProjectModule)
  );
}

// ---------------------------------------------------------------------------
// Settings route helpers
// ---------------------------------------------------------------------------

export function profileSettingsHref(tab = "general") {
  return `/settings/profile/${tab}`;
}

export function workspaceSettingsHref(workspaceSlug: string, tab?: string) {
  const base = `/workspace/${workspaceSlug}/settings`;
  return tab ? `${base}/${tab}` : base;
}

export function projectSettingsHref(
  workspaceSlug: string,
  projectId: string,
  tab?: string
) {
  const base = `/workspace/${workspaceSlug}/settings/projects/${projectId}`;
  return tab ? `${base}/${tab}` : base;
}
