import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, params }) => {
  const session = await requireSession(
    fetch,
    `/workspace/${params.workspaceSlug}`
  );

  return {
    session: session.session,
    user: session.user,
    workspaceSlug: params.workspaceSlug,
  };
};
