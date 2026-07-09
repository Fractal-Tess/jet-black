import { redirect } from "@sveltejs/kit";
import { getSession } from "$lib/server/auth";

export const load = async ({ fetch, url }) => {
  const session = await getSession(fetch);

  if (session) {
    redirect(303, getSafeReturnTo(url.searchParams.get("returnTo")));
  }

  return {
    returnTo: getSafeReturnTo(url.searchParams.get("returnTo")),
  };
};

function getSafeReturnTo(value: string | null) {
  if (!value?.startsWith("/") || value.startsWith("//")) {
    return "/dashboard";
  }

  return value;
}
