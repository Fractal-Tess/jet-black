import type { Handle } from "@sveltejs/kit";
import { withServerConvexToken } from "convex-svelte/sveltekit/server";

export const handle: Handle = async ({ event, resolve }) => {
  if (event.url.pathname.startsWith("/api/")) {
    return resolve(event);
  }

  let token: string | undefined;

  try {
    const response = await event.fetch("/api/auth/convex/token");

    if (response.ok) {
      const data = await response.json();
      token = data?.token;
    }
  } catch {
    // No token available - unauthenticated SSR
  }

  return withServerConvexToken(token, () => resolve(event));
};
