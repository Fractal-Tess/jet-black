<script lang="ts">
import ProjectLabelsTab from "$lib/components/settings/project/ProjectLabelsTab.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

let { data } = $props();
const project = $derived(data.project as { _id: string });
const membership = $derived(data.membership as { role: WorkspaceRole });
</script>

<SettingsContentWrapper>
  <ProjectLabelsTab {project} role={membership.role} />
</SettingsContentWrapper>
