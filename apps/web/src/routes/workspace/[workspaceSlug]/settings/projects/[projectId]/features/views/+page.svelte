<script lang="ts">
import LayoutGrid from "lucide-svelte/icons/layout-grid";
import ProjectFeatureToggle from "$lib/components/settings/project/ProjectFeatureToggle.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
</script>

<SettingsContentWrapper>
  <ProjectFeatureToggle
    title="Views"
    description="Save filtered and sorted views of work items for quick access."
    icon={LayoutGrid}
  />
</SettingsContentWrapper>
