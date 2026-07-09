import { describe, expect, test } from "bun:test";

import { describeServiceStatus } from "@workspace/shared";
import {
  buildHealthPayload,
  fetchConvexSummary,
  getServicePort,
} from "./index";

describe("worker", () => {
  test("formats a healthy worker message", () => {
    expect(
      describeServiceStatus({
        appName: "worker",
        environment: "test",
        healthy: true,
      })
    ).toContain("Worker");
  });

  test("uses the default worker port when no env is set", () => {
    expect(getServicePort(undefined)).toBe(4000);
  });

  test("marks the worker healthy when postgres is not configured", () => {
    expect(
      buildHealthPayload({
        convexConfigured: false,
        convexConnected: false,
        recentMessages: 0,
        environment: "test",
        databaseConfigured: false,
        databaseConnected: false,
        seededCustomers: 0,
      })
    ).toEqual({
      appName: "worker",
      convex: {
        configured: false,
        connected: false,
        recentMessages: 0,
      },
      environment: "test",
      healthy: true,
      database: {
        configured: false,
        connected: false,
        seededCustomers: 0,
      },
    });
  });

  test("uses the configured worker port", () => {
    expect(getServicePort("4123")).toBe(4123);
  });

  test("returns a disconnected Convex summary when no deployment url is set", async () => {
    await expect(fetchConvexSummary(undefined)).resolves.toEqual({
      connected: false,
      recentMessages: 0,
    });
  });
});
