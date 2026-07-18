import { api } from "@workspace/convex/api";
import { useMutation } from "convex-svelte";
import { goto } from "$app/navigation";
import type { CreateProjectInput, Project } from "$lib/components/issues/types";
import { projectModuleHref } from "$lib/routes";
import type { WorkspaceActionDeps } from "./workspace-action-deps";

export function createWorkspaceProjectActions(deps: WorkspaceActionDeps) {
  const createProject = useMutation(api.mutations.projects.create);

  let creatingProject = $state(false);

  async function onCreateProject(input: CreateProjectInput) {
    const viewerData = deps.getViewerData();

    if (!viewerData?.activeWorkspace) {
      throw new Error("Workspace is still loading.");
    }
    creatingProject = true;

    try {
      const project = await createProject({
        description: input.description,
        key: input.key,
        name: input.name,
        workspaceId: viewerData.activeWorkspace._id,
      });

      if (project) {
        deps.setSelectedIssueId(undefined);
        deps.setSelectedProjectId(project._id);
        await goto(
          projectModuleHref({
            module: "tickets",
            projectId: project._id,
            workspaceSlug: viewerData.activeWorkspace.slug,
          })
        );
        return true;
      }

      return false;
    } finally {
      creatingProject = false;
    }
  }

  async function onSelectProject(nextProjectId: Project["_id"]) {
    deps.setSelectedIssueId(undefined);
    deps.setSelectedProjectId(nextProjectId);

    const viewerData = deps.getViewerData();

    if (!viewerData?.activeWorkspace) {
      return;
    }

    await goto(
      projectModuleHref({
        module: "tickets",
        projectId: nextProjectId,
        workspaceSlug: viewerData.activeWorkspace.slug,
      })
    );
  }

  return {
    get creatingProject() {
      return creatingProject;
    },
    onCreateProject,
    onSelectProject,
  };
}
