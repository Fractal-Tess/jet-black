import { env } from "$env/dynamic/private";
import { getSession } from "$lib/server/auth";

export const load = async ({ fetch }) => {
  const session = await getSession(fetch);

  return {
    convexUrl: env.PUBLIC_CONVEX_URL ?? env.CONVEX_URL ?? "",
    isAuthenticated: Boolean(session),
  };
};
