import { error } from "@sveltejs/kit";
import { api } from "@workspace/convex/api";
import { createConvexHttpClient } from "convex-svelte/sveltekit";
import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, params }) => {
  const session = await requireSession(
    fetch,
    `/workspace/${params.workspaceSlug}/settings`
  );

  const client = createConvexHttpClient();
  const workspace = await client.query(api.queries.workspaces.workspaceBySlug, {
    slug: params.workspaceSlug,
  });

  if (!workspace) {
    error(404, "Workspace not found");
  }

  return {
    membership: workspace.membership,
    session: session.session,
    user: session.user,
    workspace: workspace.workspace,
    workspaceSlug: params.workspaceSlug,
  };
};
