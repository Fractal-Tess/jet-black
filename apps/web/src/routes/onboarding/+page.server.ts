import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, url: _url }) => {
  const session = await requireSession(fetch, "/onboarding");

  return {
    session: session.session,
    user: session.user,
  };
};
