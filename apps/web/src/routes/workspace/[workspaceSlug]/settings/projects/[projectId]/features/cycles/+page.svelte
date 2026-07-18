<script lang="ts">
import Timer from "lucide-svelte/icons/timer";
import ProjectFeatureToggle from "$lib/components/settings/project/ProjectFeatureToggle.svelte";
import SettingsContentWrapper from "$lib/components/settings/SettingsContentWrapper.svelte";
</script>

<SettingsContentWrapper>
  <ProjectFeatureToggle
    title="Sprints"
    description="Time-boxed iterations for organizing work into focused delivery periods."
    icon={Timer}
  />
</SettingsContentWrapper>
