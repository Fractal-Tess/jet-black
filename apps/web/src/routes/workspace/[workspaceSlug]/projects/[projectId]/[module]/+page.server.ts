import { error } from "@sveltejs/kit";
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { createConvexHttpClient } from "convex-svelte/sveltekit";
import { requireSession } from "$lib/server/auth";

async function preloadSsr(params: {
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

export const load = async ({ fetch, params }) => {
  const returnTo = `/workspace/${params.workspaceSlug}/projects/${params.projectId}/${params.module}`;
  const session = await requireSession(fetch, returnTo);

  let ssr: Awaited<ReturnType<typeof preloadSsr>> | undefined;

  try {
    ssr = await preloadSsr(params);
  } catch (e) {
    if (e && typeof e === "object" && "status" in e) {
      throw e;
    }
  }

  return {
    module: params.module,
    projectId: params.projectId,
    session: session.session,
    ssrIssues: ssr?.ssrIssues,
    ssrLabels: ssr?.ssrLabels,
    ssrStates: ssr?.ssrStates,
    ssrViewer: ssr?.ssrViewer,
    user: session.user,
    workspaceSlug: params.workspaceSlug,
  };
};
