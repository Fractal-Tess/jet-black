import { api } from "@workspace/convex/api";
import { ConvexHttpClient } from "convex/browser";
import { env } from "$env/dynamic/private";

export async function loadPreview(limit = 3) {
  const url = env.CONVEX_URL ?? env.PUBLIC_CONVEX_URL;
  if (!url) {
    return { messages: [], scrapes: [], connected: false };
  }

  try {
    const client = new ConvexHttpClient(url);
    const [messages, scrapes] = await Promise.all([
      client.query(api.messages.list, {}),
      client.query(api.scrapes.listRecent, { limit }),
    ]);
    return { messages, scrapes, connected: true };
  } catch {
    return { messages: [], scrapes: [], connected: false };
  }
}
