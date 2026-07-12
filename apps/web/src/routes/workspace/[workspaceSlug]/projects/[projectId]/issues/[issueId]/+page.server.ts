import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, params, url }) => {
  const session = await requireSession(fetch, url.pathname);

  return {
    issueId: params.issueId,
    module: "issues",
    projectId: params.projectId,
    session: session.session,
    user: session.user,
    workspaceSlug: params.workspaceSlug,
  };
};
