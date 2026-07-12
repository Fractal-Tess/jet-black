import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch }) => {
  const session = await requireSession(fetch, "/dashboard");

  return {
    session: session.session,
    user: session.user,
  };
};
