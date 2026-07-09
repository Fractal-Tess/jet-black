import { redirect } from "@sveltejs/kit";

type SessionUser = {
  email: string;
  id: string;
  image?: string | null;
  name?: string | null;
};

export type AuthSession = {
  session: {
    expiresAt: number | string;
    id: string;
  };
  user: SessionUser;
};

type SessionFetch = (
  input: RequestInfo | URL,
  init?: RequestInit
) => Promise<Response>;

export async function getSession(fetch: SessionFetch) {
  const response = await fetch("/api/auth/get-session");

  if (!response.ok) {
    return null;
  }

  const session: unknown = await response.json();

  if (!isAuthSession(session)) {
    return null;
  }

  return session;
}

export async function requireSession(fetch: SessionFetch, returnTo: string) {
  const session = await getSession(fetch);

  if (!session) {
    const search = new URLSearchParams({ returnTo });
    redirect(303, `/login?${search.toString()}`);
  }

  return session;
}

function isAuthSession(value: unknown): value is AuthSession {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as {
    session?: { expiresAt?: unknown; id?: unknown };
    user?: { email?: unknown; id?: unknown };
  };

  return (
    (typeof candidate.session?.expiresAt === "number" ||
      typeof candidate.session?.expiresAt === "string") &&
    typeof candidate.session.id === "string" &&
    typeof candidate.user?.email === "string" &&
    typeof candidate.user.id === "string"
  );
}
