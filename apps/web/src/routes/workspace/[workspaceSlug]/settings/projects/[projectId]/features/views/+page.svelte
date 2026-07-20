<script lang="ts">
import LayoutGrid from "lucide-svelte/icons/layout-grid";
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
    description="Save filtered and sorted views of work items for quick access."
    featureKey="views"
    icon={LayoutGrid}
    {project}
    role={membership.role}
    title="Views"
    workspaceId={workspace._id}
  />
</SettingsContentWrapper>
