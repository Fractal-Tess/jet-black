import type { Project, ViewerData } from "$lib/components/issues/types";

export type ProjectModule =
  | "intake"
  | "issues"
  | "modules"
  | "pages"
  | "sprints"
  | "views";

const DEFAULT_MODULE: ProjectModule = "issues";
const PROJECT_MODULES = [
  "intake",
  "issues",
  "modules",
  "pages",
  "sprints",
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

export function normalizeProjectModule(value?: string): ProjectModule {
  if (isProjectModule(value)) {
    return value;
  }

  return DEFAULT_MODULE;
}

export function isProjectModule(value: unknown): value is ProjectModule {
  return (
    typeof value === "string" &&
    PROJECT_MODULES.includes(value as ProjectModule)
  );
}
