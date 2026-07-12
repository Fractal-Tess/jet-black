import { getWelcomeMessage, toTitleCase } from "@workspace/shared";

export function normalizeMessageBody(body: string) {
  return body.trim().replace(/\s+/g, " ");
}

export function buildSeedMessage(appName: string) {
  return normalizeMessageBody(getWelcomeMessage(toTitleCase(appName)));
}
