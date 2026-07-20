<script lang="ts">
import { api } from "@workspace/convex/api";
import type { Id } from "@workspace/convex/dataModel";
import { useAuth, useMutation, useQuery } from "convex-svelte";
import ProjectFeatureToggle from "$lib/components/settings/project/ProjectFeatureToggle.svelte";
import {
  DEFAULT_PROJECT_FEATURES,
  type ProjectFeatureKey,
  type ProjectFeatures,
} from "$lib/project-features";
import type { SettingsNavIcon, WorkspaceRole } from "$lib/settings-nav";

let {
  project,
  workspaceId,
  role,
  featureKey,
  title,
  description,
  icon,
}: {
  project: { _id: string; features?: Partial<ProjectFeatures> };
  workspaceId: string;
  role: WorkspaceRole;
  featureKey: ProjectFeatureKey;
  title: string;
  description: string;
  icon: SettingsNavIcon;
} = $props();

const isAdmin = $derived(role === "owner" || role === "admin");
const auth = useAuth();
const projectId = $derived(project._id as Id<"projects">);

const projectQuery = useQuery(api.queries.workspaces.projectWithAccess, () =>
  auth.isAuthenticated
    ? { projectId, workspaceId: workspaceId as Id<"workspaces"> }
    : "skip"
);

const updateFeatures = useMutation(api.mutations.projects.updateFeatures);

const features = $derived({
  ...DEFAULT_PROJECT_FEATURES,
  ...(projectQuery.data?.features ?? project.features ?? {}),
});

let busy = $state(false);
let error = $state("");

async function toggle(value: boolean) {
  if (!isAdmin || busy) {
    return;
  }

  busy = true;
  error = "";
  try {
    await updateFeatures({
      features: { ...features, [featureKey]: value },
      projectId,
    });
  } catch (cause) {
    error =
      cause instanceof Error ? cause.message : "Could not update feature.";
  } finally {
    busy = false;
  }
}
</script>

<ProjectFeatureToggle
  {description}
  enabled={features[featureKey]}
  {icon}
  onToggle={toggle}
  {title}
/>

{#if error}
  <p class="mt-4 text-sm text-destructive" role="alert">{error}</p>
{/if}
