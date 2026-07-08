import { error, type RequestHandler } from "@sveltejs/kit";
import { env } from "$env/dynamic/private";

const proxy: RequestHandler = async ({ params, request, url }) => {
  const siteUrl = env.CONVEX_SITE_URL;
  if (!siteUrl) {
    error(503, "CONVEX_SITE_URL is not configured");
  }
  const target = new URL(
    `/api/auth/${params.path ?? ""}${url.search}`,
    siteUrl
  );
  const headers = new Headers(request.headers);
  headers.delete("host");
  return await fetch(target, {
    method: request.method,
    headers,
    body:
      request.method === "GET" || request.method === "HEAD"
        ? undefined
        : await request.arrayBuffer(),
    redirect: "manual",
  });
};

export const GET = proxy;
export const POST = proxy;
