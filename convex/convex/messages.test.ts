import { describe, expect, test } from "bun:test";

import { buildSeedMessage, normalizeMessageBody } from "./messages";

describe("convex messages helpers", () => {
  test("normalizes whitespace", () => {
    expect(normalizeMessageBody("  hello   convex  ")).toBe("hello convex");
  });

  test("builds a shared seed message", () => {
    expect(buildSeedMessage("web app")).toContain("Web App");
  });
});
