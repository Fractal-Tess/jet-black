import { requireSession } from "$lib/server/auth";
import { preloadProjectSsr } from "$lib/server/projectSsr";

export const load = async ({ fetch, params }) => {
  const returnTo = `/workspace/${params.workspaceSlug}/projects/${params.projectId}/${params.module}`;
  const session = await requireSession(fetch, returnTo);

  let ssr: Awaited<ReturnType<typeof preloadProjectSsr>> | undefined;

  try {
    ssr = await preloadProjectSsr(params);
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
