import { requireSession } from "$lib/server/auth";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({ fetch, params }) => {
  const session = await requireSession(
    fetch,
    `/workspace/${params.workspaceSlug}/analytics`
  );

  return {
    session: session.session,
    user: session.user,
    workspaceSlug: params.workspaceSlug,
  };
};
