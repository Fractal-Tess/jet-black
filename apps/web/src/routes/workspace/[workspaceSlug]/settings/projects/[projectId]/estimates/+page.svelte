<script lang="ts">
import ProjectEstimatesTab from "$lib/components/settings/project/ProjectEstimatesTab.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import type { EstimateSystem } from "$lib/estimates";
import type { WorkspaceRole } from "$lib/settings-nav";

let { data } = $props();
const project = $derived(
  data.project as { _id: string; estimateSystem?: EstimateSystem }
);
const workspace = $derived(data.workspace as { _id: string });
const membership = $derived(data.membership as { role: WorkspaceRole });
</script>

<SettingsContentWrapper>
  <ProjectEstimatesTab
    {project}
    role={membership.role}
    workspaceId={workspace._id}
  />
</SettingsContentWrapper>
