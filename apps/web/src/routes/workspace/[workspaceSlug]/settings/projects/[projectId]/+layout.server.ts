import { error } from "@sveltejs/kit";
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { createConvexHttpClient } from "convex-svelte/sveltekit";
import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, params }) => {
  const session = await requireSession(
    fetch,
    `/workspace/${params.workspaceSlug}/settings/projects/${params.projectId}`
  );

  const client = createConvexHttpClient();
  const workspace = await client.query(api.queries.workspaces.workspaceBySlug, {
    slug: params.workspaceSlug,
  });

  if (!workspace) {
    error(404, "Workspace not found");
  }

  const project = await client.query(api.queries.workspaces.projectWithAccess, {
    projectId: params.projectId as Id<"projects">,
    workspaceId: workspace.workspace._id,
  });

  if (!project) {
    error(404, "Project not found in this workspace");
  }

  return {
    membership: workspace.membership,
    project,
    session: session.session,
    user: session.user,
    workspace: workspace.workspace,
    workspaceSlug: params.workspaceSlug,
  };
};
