import { redirect } from "@sveltejs/kit";
import { getSession } from "$lib/server/auth";

export const load = async ({ fetch }) => {
  const session = await getSession(fetch);
  redirect(303, session ? "/dashboard" : "/login");
};
