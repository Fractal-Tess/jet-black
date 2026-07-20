<script lang="ts">
import ProjectMembersTab from "$lib/components/settings/project/ProjectMembersTab.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

let { data } = $props();
const project = $derived(data.project as { _id: string; name: string });
const workspace = $derived(data.workspace as { _id: string });
const membership = $derived(data.membership as { role: WorkspaceRole });
</script>

<SettingsContentWrapper>
  <ProjectMembersTab
    {project}
    role={membership.role}
    workspaceId={workspace._id}
  />
</SettingsContentWrapper>
