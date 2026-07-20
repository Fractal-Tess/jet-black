<script lang="ts">
import Layers from "lucide-svelte/icons/layers";
import ProjectFeaturePage from "$lib/components/settings/project/ProjectFeaturePage.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import type { ProjectFeatures } from "$lib/project-features";
import type { WorkspaceRole } from "$lib/settings-nav";

let { data } = $props();
const project = $derived(
  data.project as { _id: string; features?: Partial<ProjectFeatures> }
);
const workspace = $derived(data.workspace as { _id: string });
const membership = $derived(data.membership as { role: WorkspaceRole });
</script>

<SettingsContentWrapper>
  <ProjectFeaturePage
    description="Group related work items into logical modules for better organization."
    featureKey="modules"
    icon={Layers}
    {project}
    role={membership.role}
    title="Modules"
    workspaceId={workspace._id}
  />
</SettingsContentWrapper>
