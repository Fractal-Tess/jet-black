import { describe, expect, test } from "bun:test";

import {
  groupModuleProgress,
  normalizeModuleDate,
  normalizeModuleName,
  normalizeModuleUrl,
  validateModuleDateRange,
} from "./lib/modules";

describe("module domain helpers", () => {
  test("normalizes names and rejects blank names", () => {
    expect(normalizeModuleName("  Launch plan  ")).toBe("Launch plan");
    expect(() => normalizeModuleName("   ")).toThrow("Module name is required");
  });

  test("accepts real ISO calendar dates and normalizes empty values", () => {
    expect(normalizeModuleDate(" 2028-02-29 ")).toBe("2028-02-29");
    expect(normalizeModuleDate(null)).toBeUndefined();
    expect(normalizeModuleDate("  ")).toBeUndefined();
  });

  test("rejects malformed and impossible dates", () => {
    expect(() => normalizeModuleDate("2026-2-01")).toThrow("YYYY-MM-DD");
    expect(() => normalizeModuleDate("2026-02-29")).toThrow(
      "Module date is invalid"
    );
  });

  test("enforces an inclusive module date range", () => {
    expect(() =>
      validateModuleDateRange("2026-07-18", "2026-07-18")
    ).not.toThrow();
    expect(() => validateModuleDateRange("2026-07-19", "2026-07-18")).toThrow(
      "must not exceed"
    );
  });

  test("accepts only normalized HTTP and HTTPS links", () => {
    expect(normalizeModuleUrl(" https://example.com/docs ")).toBe(
      "https://example.com/docs"
    );
    expect(normalizeModuleUrl("http://example.com")).toBe(
      "http://example.com/"
    );
    expect(() => normalizeModuleUrl("ftp://example.com/file")).toThrow(
      "must use http or https"
    );
    expect(() => normalizeModuleUrl("not a url")).toThrow("URL is invalid");
  });

  test("groups progress by server-owned issue state type", () => {
    expect(
      groupModuleProgress([
        "backlog",
        "unstarted",
        "started",
        "started",
        "completed",
        "cancelled",
      ])
    ).toEqual({
      backlog: 1,
      cancelled: 1,
      completed: 1,
      started: 2,
      total: 6,
      unstarted: 1,
    });
  });
});
