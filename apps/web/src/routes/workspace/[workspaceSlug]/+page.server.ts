import { error } from "@sveltejs/kit";
import { api } from "@workspace/convex/api";
import { createConvexHttpClient } from "convex-svelte/sveltekit";
import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, params }) => {
  const session = await requireSession(
    fetch,
    `/workspace/${params.workspaceSlug}`
  );
  const client = createConvexHttpClient();

  const ssrWorkspace = await client.query(
    api.queries.workspaces.workspaceBySlug,
    { slug: params.workspaceSlug }
  );

  if (!ssrWorkspace) {
    error(404, "Workspace not found");
  }

  const activeProjectId = ssrWorkspace.projects[0]?._id;
  const [ssrIssues, ssrStates, ssrLabels] = activeProjectId
    ? await Promise.all([
        client.query(api.queries.issues.listForProject, {
          projectId: activeProjectId,
        }),
        client.query(api.queries.workspaces.statesForProject, {
          projectId: activeProjectId,
        }),
        client.query(api.queries.labels.listForProject, {
          projectId: activeProjectId,
        }),
      ])
    : [[], [], []];

  const ssrViewer = {
    activeProject: ssrWorkspace.projects[0] ?? null,
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

  return {
    session: session.session,
    ssrIssues,
    ssrLabels,
    ssrStates,
    ssrViewer,
    user: session.user,
    workspaceSlug: params.workspaceSlug,
  };
};
