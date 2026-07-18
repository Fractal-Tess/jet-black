import { requireSession } from "$lib/server/auth";

export const load = async ({ fetch, params }) => {
  const session = await requireSession(
    fetch,
    `/settings/profile/${params.tab}`
  );

  return {
    session: session.session,
    tab: params.tab,
    user: session.user,
  };
};
