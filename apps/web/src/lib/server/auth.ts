import { redirect } from "@sveltejs/kit";
import { z } from "zod";

const authSessionSchema = z.object({
  session: z.object({
    expiresAt: z.union([z.number(), z.string()]),
    id: z.string(),
  }),
  user: z.object({
    email: z.string(),
    id: z.string(),
    image: z.string().nullable().optional(),
    name: z.string().nullable().optional(),
  }),
});

export type AuthSession = z.infer<typeof authSessionSchema>;

type SessionFetch = (
  input: RequestInfo | URL,
  init?: RequestInit
) => Promise<Response>;

export async function getSession(fetch: SessionFetch) {
  const response = await fetch("/api/auth/get-session");

  if (!response.ok) {
    return null;
  }

  const result = authSessionSchema.safeParse(await response.json());
  return result.success ? result.data : null;
}

export async function requireSession(fetch: SessionFetch, returnTo: string) {
  const session = await getSession(fetch);

  if (!session) {
    const search = new URLSearchParams({ returnTo });
    redirect(303, `/login?${search.toString()}`);
  }

  return session;
}
