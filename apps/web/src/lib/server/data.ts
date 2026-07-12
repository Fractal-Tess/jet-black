import { api } from "@workspace/convex/api";
import { ConvexHttpClient } from "convex/browser";
import { env } from "$env/dynamic/private";

export async function loadPreview() {
  const url = env.CONVEX_URL ?? env.PUBLIC_CONVEX_URL;
  if (!url) {
    return { messages: [], connected: false };
  }

  try {
    const client = new ConvexHttpClient(url);
    const messages = await client.query(api.queries.messages.list, {});
    return { messages, connected: true };
  } catch {
    return { messages: [], connected: false };
  }
}
