<script lang="ts">
import Layers from "lucide-svelte/icons/layers";
import ProjectFeatureToggle from "$lib/components/settings/project/ProjectFeatureToggle.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
</script>

<SettingsContentWrapper>
  <ProjectFeatureToggle
    title="Modules"
    description="Group related work items into logical modules for better organization."
    icon={Layers}
  />
</SettingsContentWrapper>
