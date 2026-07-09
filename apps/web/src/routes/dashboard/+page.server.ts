import { requireSession } from "$lib/server/auth";
import { loadPreview } from "$lib/server/data";

export const load = async ({ fetch }) => {
  const session = await requireSession(fetch, "/dashboard");

  return {
    ...(await loadPreview()),
    session: session.session,
    user: session.user,
  };
};
