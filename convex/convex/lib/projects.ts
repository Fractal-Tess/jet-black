import { slugify } from "./slugs";

export function normalizeProjectKey(value: string) {
  return value
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "")
    .slice(0, 8);
}

export function buildProjectSlug(name: string, key: string) {
  return `${slugify(name) || "project"}-${key.toLowerCase()}`;
}
