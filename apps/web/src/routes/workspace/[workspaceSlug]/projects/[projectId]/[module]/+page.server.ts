import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, params }) => {
  const returnTo = `/workspace/${params.workspaceSlug}/projects/${params.projectId}/${params.module}`;
  const session = await requireSession(fetch, returnTo);

  return {
    module: params.module,
    projectId: params.projectId,
    session: session.session,
    user: session.user,
    workspaceSlug: params.workspaceSlug,
  };
};
