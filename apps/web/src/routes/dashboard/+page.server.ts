import { redirect } from "@sveltejs/kit";
import { loadPreview } from "$lib/server/data";

export const load = async ({ fetch }) => {
  const response = await fetch("/api/auth/get-session");
  if (!response.ok) {
    redirect(303, "/login");
  }
  const session = await response.json();
  if (!session?.user) {
    redirect(303, "/login");
  }
  return { ...(await loadPreview()), user: session.user };
};
