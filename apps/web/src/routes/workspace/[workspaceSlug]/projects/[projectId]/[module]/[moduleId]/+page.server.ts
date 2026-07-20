import { error } from "@sveltejs/kit";
import { requireSession } from "$lib/server/auth";
import { preloadProjectSsr } from "$lib/server/projectSsr";

export const load = async ({ fetch, params }) => {
  if (params.module !== "modules") {
    error(404, "Not found");
  }

  const returnTo = `/workspace/${params.workspaceSlug}/projects/${params.projectId}/modules/${params.moduleId}`;
  const session = await requireSession(fetch, returnTo);

  let ssr: Awaited<ReturnType<typeof preloadProjectSsr>> | undefined;

  try {
    ssr = await preloadProjectSsr({
      module: params.module,
      projectId: params.projectId,
      workspaceSlug: params.workspaceSlug,
    });
  } catch (e) {
    if (e && typeof e === "object" && "status" in e) {
      throw e;
    }
  }

  return {
    module: params.module,
    moduleId: params.moduleId,
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
