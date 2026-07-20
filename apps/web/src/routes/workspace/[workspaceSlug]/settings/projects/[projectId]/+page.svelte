<script lang="ts">
import ProjectGeneralTab from "$lib/components/settings/project/ProjectGeneralTab.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
import type { WorkspaceRole } from "$lib/settings-nav";

let { data } = $props();
const project = $derived(
  data.project as {
    _id: string;
    archivedAt?: number;
    color: string;
    coverImageUrl?: string;
    description?: string;
    key: string;
    logoUrl?: string;
    name: string;
    slug: string;
  }
);
const membership = $derived(data.membership as { role: WorkspaceRole });
const workspaceSlug = $derived(data.workspaceSlug as string);
</script>

<SettingsContentWrapper>
  <ProjectGeneralTab {project} role={membership.role} {workspaceSlug} />
</SettingsContentWrapper>
