<script lang="ts">
import ProjectGeneralTab from "$lib/components/settings/project/ProjectGeneralTab.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";

let { data } = $props();
const project = $derived(
  data.project as {
    _id: string;
    color: string;
    description?: string;
    key: string;
    name: string;
    slug: string;
  }
);
const membership = $derived(data.membership as { role: "owner" | "member" });
</script>

<SettingsContentWrapper>
  <ProjectGeneralTab {project} role={membership.role} />
</SettingsContentWrapper>
