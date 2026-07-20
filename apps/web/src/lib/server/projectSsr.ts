import { error } from "@sveltejs/kit";
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { createConvexHttpClient } from "convex-svelte/sveltekit";

export async function preloadProjectSsr(params: {
  module: string;
  projectId: string;
  workspaceSlug: string;
}) {
  const client = createConvexHttpClient();

  const ssrWorkspace = await client.query(
    api.queries.workspaces.workspaceBySlug,
    { slug: params.workspaceSlug }
  );

  if (!ssrWorkspace) {
    error(404, "Workspace not found");
  }

  const ssrProject = await client.query(
    api.queries.workspaces.projectWithAccess,
    {
      projectId: params.projectId as Id<"projects">,
      workspaceId: ssrWorkspace.workspace._id,
    }
  );

  if (!ssrProject) {
    error(404, "Project not found in this workspace");
  }

  const [ssrIssues, ssrStates, ssrLabels] = await Promise.all([
    client.query(api.queries.issues.listForProject, {
      projectId: params.projectId as Id<"projects">,
    }),
    client.query(api.queries.workspaces.statesForProject, {
      projectId: params.projectId as Id<"projects">,
    }),
    client.query(api.queries.labels.listForProject, {
      projectId: params.projectId as Id<"projects">,
    }),
  ]);

  const ssrViewer = {
    activeProject: ssrProject ?? ssrWorkspace.projects[0] ?? null,
    activeWorkspace: ssrWorkspace.workspace,
    memberships: [
      {
        role: ssrWorkspace.membership.role,
        workspaceId: ssrWorkspace.membership.workspaceId,
      },
    ],
    projects: ssrWorkspace.projects,
    user: ssrWorkspace.user,
    workspaces: [ssrWorkspace.workspace],
  };

  return { ssrIssues, ssrLabels, ssrStates, ssrViewer };
}
