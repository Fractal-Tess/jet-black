export type ProjectFeatureKey =
  | "intake"
  | "modules"
  | "pages"
  | "sprints"
  | "views";

export type ProjectFeatures = Record<ProjectFeatureKey, boolean>;

export const DEFAULT_PROJECT_FEATURES: ProjectFeatures = {
  intake: true,
  modules: true,
  pages: true,
  sprints: true,
  views: true,
};

/** Maps app-shell module keys to their controlling project feature flag. */
export const MODULE_FEATURE: Record<string, ProjectFeatureKey> = {
  intake: "intake",
  modules: "modules",
  pages: "pages",
  sprints: "sprints",
  views: "views",
};

export function isModuleEnabled(
  moduleKey: string,
  features: Partial<ProjectFeatures> | undefined | null
): boolean {
  const featureKey = MODULE_FEATURE[moduleKey];

  if (!featureKey) {
    return true;
  }

  return features?.[featureKey] ?? true;
}
