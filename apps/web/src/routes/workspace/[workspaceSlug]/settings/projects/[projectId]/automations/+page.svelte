<script lang="ts">
import ProjectAutomationsTab from "$lib/components/settings/project/ProjectAutomationsTab.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

type Automations = {
  autoArchiveClosedMonths: number | null;
  autoCloseInactiveMonths: number | null;
};

let { data } = $props();
const project = $derived(
  data.project as { _id: string; automations?: Automations }
);
const workspace = $derived(data.workspace as { _id: string });
const membership = $derived(data.membership as { role: WorkspaceRole });
</script>

<SettingsContentWrapper>
  <ProjectAutomationsTab
    {project}
    role={membership.role}
    workspaceId={workspace._id}
  />
</SettingsContentWrapper>
